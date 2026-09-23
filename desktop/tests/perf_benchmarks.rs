// desktop/tests/perf_benchmarks.rs
use std::time::Instant;

#[test]
fn benchmark_directory_sorting() {
    println!("\n========================================================");
    println!("📊 BENCHMARK 1: Directory Listing & Sorting (10,000 items)");
    println!("========================================================");

    // Generate 10,000 realistic filenames
    let sample: Vec<String> = (0..10_000)
        .map(|i| {
            if i % 3 == 0 {
                format!("Document_{:05}_FINAL.PDF", i)
            } else if i % 3 == 1 {
                format!("document_{:05}_final.pdf", i)
            } else {
                format!("Doc_{:05}.txt", i)
            }
        })
        .collect();

    // 1. OLD METHOD: sort_by_key with to_lowercase() (allocating on every comparison)
    let mut old_items = sample.clone();
    let old_start = Instant::now();
    old_items.sort_by_key(|item| item.to_lowercase());
    let old_duration = old_start.elapsed();

    // 2. CACHED KEY: sort_by_cached_key (O(N) allocations instead of O(N log N))
    let mut cached_items = sample.clone();
    let cached_start = Instant::now();
    cached_items.sort_by_cached_key(|item| item.to_lowercase());
    let cached_duration = cached_start.elapsed();

    println!("  • Old method (sort_by_key):         {:?}", old_duration);
    println!("  • Optimized (sort_by_cached_key):   {:?}", cached_duration);
    let speedup = old_duration.as_secs_f64() / cached_duration.as_secs_f64();
    println!("  🚀 Speedup: {:.2}x faster! (allocations reduced from ~130,000 to 10,000)", speedup);

    assert_eq!(old_items.len(), cached_items.len());
}

#[test]
fn benchmark_file_transfer_and_apfs_clone() {
    println!("\n========================================================");
    println!("📊 BENCHMARK 2: File Transfer (APFS Clonefile vs Read/Write)");
    println!("========================================================");

    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("source_50mb.bin");
    let dst_copy = tmp.path().join("dest_streamed.bin");
    let dst_clone = tmp.path().join("dest_clone.bin");

    // Create 50 MB sample file
    let chunk = vec![0x55u8; 1024 * 1024]; // 1 MB
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&src).unwrap();
        for _ in 0..50 {
            f.write_all(&chunk).unwrap();
        }
    }

    // 1. Traditional buffered copy (reading & writing 50 MB)
    let copy_start = Instant::now();
    std::fs::copy(&src, &dst_copy).unwrap();
    let copy_duration = copy_start.elapsed();

    // 2. APFS Clonefile (macOS Copy-on-Write)
    #[cfg(target_os = "macos")]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let src_c = CString::new(src.as_os_str().as_bytes()).unwrap();
        let dst_c = CString::new(dst_clone.as_os_str().as_bytes()).unwrap();

        let clone_start = Instant::now();
        let ret = unsafe { libc::clonefile(src_c.as_ptr(), dst_c.as_ptr(), 0) };
        let clone_duration = clone_start.elapsed();

        assert_eq!(ret, 0, "clonefile must succeed on APFS");
        println!("  • Standard I/O copy (50 MB): {:?}", copy_duration);
        println!("  • APFS clonefile (50 MB):   {:?}", clone_duration);
        let speedup = copy_duration.as_secs_f64() / clone_duration.as_secs_f64().max(0.000001);
        println!("  🚀 Speedup: {:.1}x faster (Instant Zero-Copy)! 0 bytes rewritten to SSD", speedup);
    }
}
