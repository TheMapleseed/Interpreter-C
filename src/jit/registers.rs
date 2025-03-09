// src/jit/registers.rs
use std::collections::{HashMap, HashSet, VecDeque};
use bitflags::bitflags;

bitflags! {
    pub struct RegisterClass: u32 {
        const GENERAL = 0b0001;  // General purpose registers
        const VECTOR  = 0b0010;  // Vector/SIMD registers
        const FLOAT   = 0b0100;  // Floating point registers
        const SPECIAL = 0b1000;  // Special purpose registers
    }
}

pub struct RegisterAllocator {
    // Register state tracking
    allocated: HashMap<VirtualReg, PhysicalReg>,
    available: HashMap<RegisterClass, VecDeque<PhysicalReg>>,
    
    // Spill management
    stack_slots: HashMap<VirtualReg, StackSlot>,
    next_stack_slot: i32,
    
    // Register interference
    interference_graph: InterferenceGraph,
    
    // ABI handling
    abi_reserved: HashSet<PhysicalReg>,
    callee_saved: HashSet<PhysicalReg>,
}

impl RegisterAllocator {
    pub fn new() -> Self {
        let mut allocator = RegisterAllocator {
            allocated: HashMap::new(),
            available: HashMap::new(),
            stack_slots: HashMap::new(),
            next_stack_slot: 0,
            interference_graph: InterferenceGraph::new(),
            abi_reserved: HashSet::new(),
            callee_saved: HashSet::new(),
        };

        // Initialize register pools
        allocator.initialize_registers();
        allocator
    }

    fn initialize_registers(&mut self) {
        // General purpose registers
        let mut general = VecDeque::new();
        general.extend([
            PhysicalReg::RAX,
            PhysicalReg::RBX,
            PhysicalReg::RCX,
            PhysicalReg::RDX,
            PhysicalReg::RSI,
            PhysicalReg::RDI,
            PhysicalReg::R8,
            PhysicalReg::R9,
            PhysicalReg::R10,
            PhysicalReg::R11,
            PhysicalReg::R12,
            PhysicalReg::R13,
            PhysicalReg::R14,
            PhysicalReg::R15,
        ]);
        self.available.insert(RegisterClass::GENERAL, general);

        // Vector/SIMD registers
        let mut vector = VecDeque::new();
        vector.extend([
            PhysicalReg::XMM0,
            PhysicalReg::XMM1,
            PhysicalReg::XMM2,
            PhysicalReg::XMM3,
            PhysicalReg::XMM4,
            PhysicalReg::XMM5,
            PhysicalReg::XMM6,
            PhysicalReg::XMM7,
            PhysicalReg::XMM8,
            PhysicalReg::XMM9,
            PhysicalReg::XMM10,
            PhysicalReg::XMM11,
            PhysicalReg::XMM12,
            PhysicalReg::XMM13,
            PhysicalReg::XMM14,
            PhysicalReg::XMM15,
        ]);
        self.available.insert(RegisterClass::VECTOR, vector);

        // Set up ABI registers
        self.setup_abi_registers();
    }

    fn setup_abi_registers(&mut self) {
        // System V AMD64 ABI
        
        // Caller-saved registers
        let caller_saved = [
            PhysicalReg::RAX,  // Return value
            PhysicalReg::RCX,  // 4th argument
            PhysicalReg::RDX,  // 3rd argument
            PhysicalReg::RSI,  // 2nd argument
            PhysicalReg::RDI,  // 1st argument
            PhysicalReg::R8,   // 5th argument
            PhysicalReg::R9,   // 6th argument
            PhysicalReg::R10,
            PhysicalReg::R11,
        ];

        // Callee-saved registers
        self.callee_saved.extend([
            PhysicalReg::RBX,
            PhysicalReg::RBP,
            PhysicalReg::R12,
            PhysicalReg::R13,
            PhysicalReg::R14,
            PhysicalReg::R15,
        ]);

        // Reserved registers
        self.abi_reserved.extend([
            PhysicalReg::RSP,  // Stack pointer
            PhysicalReg::RBP,  // Frame pointer
        ]);
    }

    pub fn allocate(
        &mut self, 
        vreg: VirtualReg,
        class: RegisterClass
    ) -> Result<PhysicalReg, AllocError> {
        // Check if already allocated
        if let Some(&preg) = self.allocated.get(&vreg) {
            return Ok(preg);
        }

        // Try to get a free register
        if let Some(preg) = self.get_free_register(class) {
            self.allocated.insert(vreg, preg);
            return Ok(preg);
        }

        // Need to spill
        self.spill_register(vreg, class)
    }

    pub fn free(&mut self, vreg: VirtualReg) {
        if let Some(preg) = self.allocated.remove(&vreg) {
            // Return to appropriate pool
            let class = preg.register_class();
            if let Some(pool) = self.available.get_mut(&class) {
                pool.push_back(preg);
            }
        }
    }

    fn get_free_register(&mut self, class: RegisterClass) -> Option<PhysicalReg> {
        self.available.get_mut(&class)?.pop_front()
    }

    fn spill_register(
        &mut self,
        vreg: VirtualReg,
        class: RegisterClass
    ) -> Result<PhysicalReg, AllocError> {
        // Find best candidate for spilling
        let spill_candidate = self.find_spill_candidate(class)?;
        
        // Allocate stack slot if needed
        let stack_slot = self.get_or_create_stack_slot(vreg);
        
        // Generate spill code
        self.generate_spill_code(spill_candidate, stack_slot)?;
        
        // Update allocations
        let spilled_vreg = self.get_vreg_from_preg(spill_candidate)
            .ok_or(AllocError::InvalidRegister)?;
        self.allocated.remove(&spilled_vreg);
        self.allocated.insert(vreg, spill_candidate);
        
        Ok(spill_candidate)
    }

    fn find_spill_candidate(&self, class: RegisterClass) -> Result<PhysicalReg, AllocError> {
        // Use interference graph to find best candidate
        let mut best_score = f64::MAX;
        let mut best_reg = None;

        for (&vreg, &preg) in &self.allocated {
            if preg.register_class() != class {
                continue;
            }

            let score = self.calculate_spill_score(vreg);
            if score < best_score {
                best_score = score;
                best_reg = Some(preg);
            }
        }

        best_reg.ok_or(AllocError::NoSpillCandidate)
    }

    fn calculate_spill_score(&self, vreg: VirtualReg) -> f64 {
        // Calculate spill priority based on:
        // - Number of uses
        // - Distance to next use
        // - Register pressure
        // - Interference degree
        let uses = self.interference_graph.get_use_count(vreg);
        let next_use = self.interference_graph.get_next_use(vreg);
        let interference = self.interference_graph.get_interference_degree(vreg);

        let use_score = uses as f64;
        let distance_score = next_use.map_or(1000.0, |d| d as f64);
        let interference_score = interference as f64;

        // Weighted formula for spill priority
        (interference_score * 0.5) + (distance_score * 0.3) - (use_score * 0.2)
    }

    fn get_or_create_stack_slot(&mut self, vreg: VirtualReg) -> StackSlot {
        if let Some(&slot) = self.stack_slots.get(&vreg) {
            return slot;
        }

        let new_slot = StackSlot {
            offset: self.next_stack_slot,
            size: vreg.size(),
        };
        self.next_stack_slot += new_slot.size;
        self.stack_slots.insert(vreg, new_slot);
        new_slot
    }

    fn generate_spill_code(
        &self,
        reg: PhysicalReg,
        slot: StackSlot
    ) -> Result<(), AllocError> {
        // Generate store/load instructions for spilling
        // This will be implemented by the code generator
        Ok(())
    }

    pub fn get_frame_size(&self) -> i32 {
        // Align to 16 bytes
        (self.next_stack_slot + 15) & !15
    }

    pub fn get_callee_saved(&self) -> &HashSet<PhysicalReg> {
        &self.callee_saved
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhysicalReg {
    // General purpose
    RAX, RBX, RCX, RDX,
    RSI, RDI, RSP, RBP,
    R8,  R9,  R10, R11,
    R12, R13, R14, R15,
    
    // Vector/SIMD
    XMM0,  XMM1,  XMM2,  XMM3,
    XMM4,  XMM5,  XMM6,  XMM7,
    XMM8,  XMM9,  XMM10, XMM11,
    XMM12, XMM13, XMM14, XMM15,
}

impl PhysicalReg {
    fn register_class(&self) -> RegisterClass {
        match self {
            PhysicalReg::RAX..=PhysicalReg::R15 => RegisterClass::GENERAL,
            PhysicalReg::XMM0..=PhysicalReg::XMM15 => RegisterClass::VECTOR,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VirtualReg {
    id: u32,
    class: RegisterClass,
    size: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct StackSlot {
    offset: i32,
    size: i32,
}

struct InterferenceGraph {
    edges: HashMap<VirtualReg, HashSet<VirtualReg>>,
    use_counts: HashMap<VirtualReg, usize>,
    next_uses: HashMap<VirtualReg, usize>,
}

impl InterferenceGraph {
    fn new() -> Self {
        InterferenceGraph {
            edges: HashMap::new(),
            use_counts: HashMap::new(),
            next_uses: HashMap::new(),
        }
    }

    fn add_interference(&mut self, a: VirtualReg, b: VirtualReg) {
        self.edges.entry(a).or_default().insert(b);
        self.edges.entry(b).or_default().insert(a);
    }

    fn get_interference_degree(&self, reg: VirtualReg) -> usize {
        self.edges.get(&reg).map_or(0, |edges| edges.len())
    }

    fn get_use_count(&self, reg: VirtualReg) -> usize {
        self.use_counts.get(&reg).copied().unwrap_or(0)
    }

    fn get_next_use(&self, reg: VirtualReg) -> Option<usize> {
        self.next_uses.get(&reg).copied()
    }
}

#[derive(Debug)]
pub enum AllocError {
    NoAvailableRegisters,
    NoSpillCandidate,
    InvalidRegister,
    SpillFailed,
}

// Example usage:
/*
fn main() -> Result<(), AllocError> {
    let mut allocator = RegisterAllocator::new();

    let vreg1 = VirtualReg { id: 1, class: RegisterClass::GENERAL, size: 8 };
    let vreg2 = VirtualReg { id: 2, class: RegisterClass::GENERAL, size: 8 };

    // Allocate registers
    let preg1 = allocator.allocate(vreg1, RegisterClass::GENERAL)?;
    let preg2 = allocator.allocate(vreg2, RegisterClass::GENERAL)?;

    println!("Virtual register 1 allocated to {:?}", preg1);
    println!("Virtual register 2 allocated to {:?}", preg2);

    // Free registers
    allocator.free(vreg1);
    allocator.free(vreg2);

    Ok(())
}
*/
