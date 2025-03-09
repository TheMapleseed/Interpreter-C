// src/runtime/stack.rs
use std::collections::HashMap;
use std::sync::Arc;

pub struct StackManager {
    // Frame management
    current_frame: Option<StackFrame>,
    frame_cache: Vec<StackFrame>,
    
    // Stack layout
    layout_manager: StackLayoutManager,
    
    // Spill handling
    spill_slots: HashMap<VirtualReg, SpillSlot>,
    
    // Stack probing and protection
    guard_manager: StackGuardManager,
    
    // Stack unwinding info
    unwind_info: UnwindInfoTable,
}

impl StackManager {
    pub fn new(config: StackConfig) -> Result<Self, StackError> {
        Ok(StackManager {
            current_frame: None,
            frame_cache: Vec::new(),
            layout_manager: StackLayoutManager::new(config.alignment),
            spill_slots: HashMap::new(),
            guard_manager: StackGuardManager::new(config.guard_size)?,
            unwind_info: UnwindInfoTable::new(),
        })
    }

    pub unsafe fn create_frame(&mut self, func: &Function) -> Result<StackFrameToken, StackError> {
        // Calculate frame requirements
        let frame_size = self.calculate_frame_size(func)?;
        
        // Check stack space and probe if needed
        self.guard_manager.check_stack_space(frame_size)?;
        
        // Get or create frame
        let frame = self.get_or_create_frame(frame_size)?;
        
        // Setup frame
        self.setup_frame(&frame, func)?;
        
        // Generate unwind info
        let unwind_token = self.unwind_info.register_frame(&frame)?;
        
        // Return frame token
        Ok(StackFrameToken {
            frame_id: frame.id,
            unwind_token,
        })
    }

    unsafe fn calculate_frame_size(&self, func: &Function) -> Result<usize, StackError> {
        let mut size = 0;

        // Space for saved registers
        size += self.layout_manager.saved_regs_size(func);
        
        // Local variables
        size += func.locals.iter().map(|local| local.size()).sum::<usize>();
        
        // Spill slots
        size += self.spill_slots.values()
            .map(|slot| slot.size)
            .sum::<usize>();
            
        // Align to required boundary
        size = self.layout_manager.align_size(size);

        Ok(size)
    }

    unsafe fn setup_frame(
        &mut self,
        frame: &StackFrame,
        func: &Function
    ) -> Result<(), StackError> {
        // Save return address
        self.save_return_address(frame)?;
        
        // Save callee-saved registers
        self.save_registers(frame, func.saved_regs())?;
        
        // Setup frame pointer
        self.setup_frame_pointer(frame)?;
        
        // Allocate locals
        self.allocate_locals(frame, &func.locals)?;
        
        // Setup spill slots
        self.setup_spill_slots(frame)?;

        Ok(())
    }

    unsafe fn save_registers(
        &mut self,
        frame: &StackFrame,
        regs: &[Register]
    ) -> Result<(), StackError> {
        for reg in regs {
            let offset = self.layout_manager.get_register_slot(*reg)?;
            frame.save_register(*reg, offset)?;
        }
        Ok(())
    }

    pub unsafe fn allocate_spill_slot(
        &mut self,
        vreg: VirtualReg,
        size: usize
    ) -> Result<SpillSlot, StackError> {
        // Check if already allocated
        if let Some(slot) = self.spill_slots.get(&vreg) {
            return Ok(*slot);
        }

        // Allocate new slot
        let offset = self.layout_manager.allocate_spill_slot(size)?;
        
        let slot = SpillSlot {
            offset,
            size,
        };
        
        self.spill_slots.insert(vreg, slot);
        Ok(slot)
    }

    pub unsafe fn spill_register(
        &mut self,
        reg: Register,
        slot: SpillSlot
    ) -> Result<(), StackError> {
        let frame = self.current_frame
            .as_ref()
            .ok_or(StackError::NoCurrentFrame)?;
            
        frame.store_register(reg, slot.offset)?;
        Ok(())
    }

    pub unsafe fn reload_register(
        &mut self,
        reg: Register,
        slot: SpillSlot
    ) -> Result<(), StackError> {
        let frame = self.current_frame
            .as_ref()
            .ok_or(StackError::NoCurrentFrame)?;
            
        frame.load_register(reg, slot.offset)?;
        Ok(())
    }

    pub unsafe fn destroy_frame(&mut self, token: StackFrameToken) -> Result<(), StackError> {
        // Restore callee-saved registers
        if let Some(frame) = self.current_frame.as_ref() {
            self.restore_registers(frame)?;
        }
        
        // Remove unwind info
        self.unwind_info.deregister_frame(token.unwind_token)?;
        
        // Cache frame for reuse
        if let Some(frame) = self.current_frame.take() {
            self.frame_cache.push(frame);
        }
        
        Ok(())
    }
}

struct StackLayoutManager {
    alignment: usize,
    current_offset: usize,
    register_slots: HashMap<Register, usize>,
}

impl StackLayoutManager {
    fn new(alignment: usize) -> Self {
        StackLayoutManager {
            alignment,
            current_offset: 0,
            register_slots: HashMap::new(),
        }
    }

    fn allocate_slot(&mut self, size: usize) -> usize {
        let offset = self.align(self.current_offset);
        self.current_offset = offset + size;
        offset
    }

    fn align(&self, size: usize) -> usize {
        (size + self.alignment - 1) & !(self.alignment - 1)
    }

    fn get_register_slot(&mut self, reg: Register) -> Result<usize, StackError> {
        if let Some(&slot) = self.register_slots.get(&reg) {
            return Ok(slot);
        }

        let slot = self.allocate_slot(reg.size());
        self.register_slots.insert(reg, slot);
        Ok(slot)
    }
}

struct StackGuardManager {
    guard_size: usize,
    probe_size: usize,
}

impl StackGuardManager {
    fn new(guard_size: usize) -> Result<Self, StackError> {
        Ok(StackGuardManager {
            guard_size,
            probe_size: 4096,  // Page size
        })
    }

    unsafe fn check_stack_space(&self, size: usize) -> Result<(), StackError> {
        // Probe stack in page-size increments
        let mut current = 0;
        while current < size {
            let probe_addr = std::ptr::read_volatile(
                (std::ptr::null::<u8>() as usize - current) as *const u8
            );
            current += self.probe_size;
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct SpillSlot {
    offset: usize,
    size: usize,
}

struct StackFrame {
    id: usize,
    base: *mut u8,
    size: usize,
    saved_registers: Vec<(Register, usize)>,
}

impl StackFrame {
    unsafe fn new(size: usize) -> Result<Self, StackError> {
        // Allocate stack space
        let base = libc::alloca(size) as *mut u8;
        if base.is_null() {
            return Err(StackError::AllocationFailed);
        }

        Ok(StackFrame {
            id: generate_frame_id(),
            base,
            size,
            saved_registers: Vec::new(),
        })
    }

    unsafe fn save_register(&mut self, reg: Register, offset: usize) -> Result<(), StackError> {
        let addr = self.base.add(offset);
        std::ptr::write(addr as *mut u64, reg.read());
        self.saved_registers.push((reg, offset));
        Ok(())
    }

    unsafe fn load_register(&self, reg: Register, offset: usize) -> Result<(), StackError> {
        let addr = self.base.add(offset);
        reg.write(std::ptr::read(addr as *const u64));
        Ok(())
    }
}

#[derive(Debug)]
pub enum StackError {
    AllocationFailed,
    NoCurrentFrame,
    InvalidOffset,
    UnwindError(String),
    GuardError(String),
}

// Example usage:
/*
unsafe fn example() -> Result<(), StackError> {
    let mut stack_manager = StackManager::new(StackConfig {
        alignment: 16,
        guard_size: 4096,
    })?;

    // Create new stack frame
    let func = Function::new();  // Your function info
    let frame_token = stack_manager.create_frame(&func)?;

    // Allocate spill slot
    let vreg = VirtualReg::new(1);
    let slot = stack_manager.allocate_spill_slot(vreg, 8)?;

    // Spill register
    stack_manager.spill_register(Register::RAX, slot)?;

    // Later, reload register
    stack_manager.reload_register(Register::RAX, slot)?;

    // Cleanup
    stack_manager.destroy_frame(frame_token)?;

    Ok(())
}
*/
