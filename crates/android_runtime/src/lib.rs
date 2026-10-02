#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "android")]
#[global_allocator]
static PROCESS_ALLOCATOR: diag::ProcessCountingAllocator = diag::ProcessCountingAllocator;
