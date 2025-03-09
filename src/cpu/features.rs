// src/cpu/features.rs
use std::sync::Arc;
use bitflags::bitflags;
use raw_cpuid::CpuId;
use std::arch::x86_64::*;

bitflags! {
    pub struct CPUFeatures: u64 {
        const SSE       = 1 << 0;
        const SSE2      = 1 << 1;
        const SSE3      = 1 << 2;
        const SSSE3     = 1 << 3;
        const SSE4_1    = 1 << 4;
        const SSE4_2    = 1 << 5;
        const AVX       = 1 << 6;
        const AVX2      = 1 << 7;
        const FMA       = 1 << 8;
        const BMI1      = 1 << 9;
        const BMI2      = 1 << 10;
        const POPCNT    = 1 << 11;
        const LZCNT     = 1 << 12;
        const MOVBE     = 1 << 13;
        const AES       = 1 << 14;
        const AVX512F   = 1 << 15;
        const AVX512VL  = 1 << 16;
        const AVX512BW  = 1 << 17;
        const AVX512DQ  = 1 << 18;
        const ADX       = 1 << 19;
        const PREFETCH  = 1 << 20;
    }
}

pub struct CPUInfo {
    // Core feature detection
    features: CPUFeatures,
    
    // CPU identification
    vendor: Vendor,
    brand: String,
    
    // Cache information
    cache_info: CacheInfo,
    
    // Microarchitecture details
    uarch: Microarchitecture,
    
    // Performance characteristics
    perf_info: PerfInfo,
}

impl CPUInfo {
    pub fn new() -> Result<Self, CPUError> {
        let cpuid = CpuId::new();
        
        // Get basic vendor info
        let vendor = Self::detect_vendor(&cpuid)?;
        let brand = Self::get_brand_string(&cpuid)?;
        
        // Detect features
        let features = Self::detect_features(&cpuid)?;
        
        // Get cache information
        let cache_info = Self::detect_cache_info(&cpuid)?;
        
        // Determine microarchitecture
        let uarch = Self::detect_microarchitecture(&cpuid, vendor)?;
        
        // Gather performance info
        let perf_info = Self::gather_perf_info(&cpuid, &uarch)?;
        
        Ok(CPUInfo {
            features,
            vendor,
            brand,
            cache_info,
            uarch,
            perf_info,
        })
    }

    fn detect_features(cpuid: &CpuId) -> Result<CPUFeatures, CPUError> {
        let mut features = CPUFeatures::empty();
        
        if let Some(info) = cpuid.get_feature_info() {
            if info.has_sse() { features |= CPUFeatures::SSE; }
            if info.has_sse2() { features |= CPUFeatures::SSE2; }
            if info.has_sse3() { features |= CPUFeatures::SSE3; }
            if info.has_ssse3() { features |= CPUFeatures::SSSE3; }
            if info.has_sse41() { features |= CPUFeatures::SSE4_1; }
            if info.has_sse42() { features |= CPUFeatures::SSE4_2; }
            if info.has_avx() { features |= CPUFeatures::AVX; }
        }

        if let Some(info) = cpuid.get_extended_feature_info() {
            if info.has_avx2() { features |= CPUFeatures::AVX2; }
            if info.has_bmi1() { features |= CPUFeatures::BMI1; }
            if info.has_bmi2() { features |= CPUFeatures::BMI2; }
            if info.has_avx512f() { features |= CPUFeatures::AVX512F; }
            if info.has_avx512vl() { features |= CPUFeatures::AVX512VL; }
            if info.has_avx512bw() { features |= CPUFeatures::AVX512BW; }
            if info.has_avx512dq() { features |= CPUFeatures::AVX512DQ; }
        }

        Ok(features)
    }

    pub fn supports(&self, feature: CPUFeatures) -> bool {
        self.features.contains(feature)
    }

    pub fn best_simd_width(&self) -> SimdWidth {
        if self.supports(CPUFeatures::AVX512F) {
            SimdWidth::AVX512
        } else if self.supports(CPUFeatures::AVX2) {
            SimdWidth::AVX2
        } else if self.supports(CPUFeatures::AVX) {
            SimdWidth::AVX
        } else if self.supports(CPUFeatures::SSE4_2) {
            SimdWidth::SSE4
        } else if self.supports(CPUFeatures::SSE2) {
            SimdWidth::SSE2
        } else {
            SimdWidth::Scalar
        }
    }

    pub fn optimal_instruction_set(&self) -> InstructionSet {
        let mut set = InstructionSet::new();
        
        // Base instruction selection
        if self.supports(CPUFeatures::BMI2) {
            set.mulx = true;
            set.pdep = true;
            set.pext = true;
        }
        
        if self.supports(CPUFeatures::BMI1) {
            set.tzcnt = true;
            set.lzcnt = true;
        }
        
        if self.supports(CPUFeatures::ADX) {
            set.adcx = true;
            set.adox = true;
        }

        // Vector instruction selection
        set.vector_width = self.best_simd_width();
        
        if self.supports(CPUFeatures::FMA) {
            set.fma = true;
        }

        set
    }

    fn detect_cache_info(cpuid: &CpuId) -> Result<CacheInfo, CPUError> {
        let mut cache_info = CacheInfo::default();
        
        if let Some(info) = cpuid.get_cache_info() {
            for cache in info {
                match cache.level() {
                    1 if cache.cache_type().is_data() => {
                        cache_info.l1d_size = cache.size() as u32;
                        cache_info.l1d_line_size = cache.line_size() as u32;
                        cache_info.l1d_associativity = cache.associativity();
                    },
                    2 => {
                        cache_info.l2_size = cache.size() as u32;
                        cache_info.l2_line_size = cache.line_size() as u32;
                        cache_info.l2_associativity = cache.associativity();
                    },
                    3 => {
                        cache_info.l3_size = cache.size() as u32;
                        cache_info.l3_line_size = cache.line_size() as u32;
                        cache_info.l3_associativity = cache.associativity();
                    },
                    _ => {}
                }
            }
        }

        Ok(cache_info)
    }

    pub fn get_cache_info(&self) -> &CacheInfo {
        &self.cache_info
    }

    pub fn suggest_prefetch_distance(&self) -> u32 {
        // Calculate optimal prefetch distance based on cache characteristics
        let line_size = self.cache_info.l1d_line_size;
        match self.uarch {
            Microarchitecture::Skylake | 
            Microarchitecture::CascadeLake |
            Microarchitecture::IceLake => line_size * 4,
            Microarchitecture::Zen | 
            Microarchitecture::Zen2 |
            Microarchitecture::Zen3 => line_size * 3,
            _ => line_size * 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstructionSet {
    // Vector instructions
    vector_width: SimdWidth,
    fma: bool,
    
    // BMI instructions
    mulx: bool,
    pdep: bool,
    pext: bool,
    tzcnt: bool,
    lzcnt: bool,
    
    // ADX instructions
    adcx: bool,
    adox: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimdWidth {
    Scalar,
    SSE2,
    SSE4,
    AVX,
    AVX2,
    AVX512,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    Intel,
    AMD,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Microarchitecture {
    Unknown,
    // Intel
    Skylake,
    CascadeLake,
    IceLake,
    // AMD
    Zen,
    Zen2,
    Zen3,
}

#[derive(Debug, Default)]
pub struct CacheInfo {
    l1d_size: u32,
    l1d_line_size: u32,
    l1d_associativity: u32,
    l2_size: u32,
    l2_line_size: u32,
    l2_associativity: u32,
    l3_size: u32,
    l3_line_size: u32,
    l3_associativity: u32,
}

#[derive(Debug)]
pub struct PerfInfo {
    uops_per_cycle: f32,
    ports: u32,
    pipeline_depth: u32,
    branch_predictor_size: u32,
}

#[derive(Debug)]
pub enum CPUError {
    FeatureDetectionFailed,
    UnsupportedCPU,
    InvalidVendor,
}

// Example usage:
/*
fn main() -> Result<(), CPUError> {
    let cpu_info = CPUInfo::new()?;

    // Check available features
    println!("CPU Features: {:?}", cpu_info.features);

    // Get optimal instruction set
    let inst_set = cpu_info.optimal_instruction_set();
    println!("Optimal SIMD width: {:?}", inst_set.vector_width);

    // Get cache information
    let cache_info = cpu_info.get_cache_info();
    println!("L1D Cache Size: {} KB", cache_info.l1d_size / 1024);

    // Get prefetch hints
    let prefetch_distance = cpu_info.suggest_prefetch_distance();
    println!("Suggested prefetch distance: {} bytes", prefetch_distance);

    Ok(())
}
*/
