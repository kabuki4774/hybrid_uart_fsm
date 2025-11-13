# Fuzz Report
_Generated: Thu Nov 13 12:18:54 EST 2025_

- cargo-fuzz cargo-fuzz 0.13.1

## Targets & settings
- FUZZ_SECONDS: 30
- FUZZ_FEATURES: std,crc16,crc16_2b,byte_stuff (via fuzz/Cargo.toml for rust_logic)
- FUZZ_TARGETS: fuzz_parser fuzz_diff

### fuzz_parser
- ✅ Completed without crashes
- Corpus: 906 → 1113 files
- Artifacts: 0 files in `fuzz/artifacts/fuzz_parser`

<details><summary>Final libFuzzer stats (tail)</summary>

```
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
Rust(worker): Unknown TYPE 0xFD
#1023657	DONE   cov: 71 ft: 317 corp: 128/28Kb lim: 4096 exec/s: 33021 rss: 164Mb
###### Recommended dictionary. ######
"\000\000" # Uses: 12578
"\377\377\377\377" # Uses: 7140
"\007\000\000\000\000\000\000\000" # Uses: 6860
"\001\000" # Uses: 3775
"\000\000\000\000\000\000\000\000" # Uses: 3344
"\000\000\000\000" # Uses: 1943
"@\000\000\000\000\000\000\000" # Uses: 1304
"\377\377" # Uses: 1124
"\000\000\000\000\000\000\000@" # Uses: 965
"\006\000\000\000\000\000\000\000" # Uses: 703
"\004\000\000\000\000\000\000\000" # Uses: 440
"\001\000\000\000\000\000\000\013" # Uses: 126
###### End of recommended dictionary. ######
Done 1023657 runs in 31 second(s)
stat::number_of_executed_units: 1023657
stat::average_exec_per_sec:     33021
stat::new_units_added:          273
stat::slowest_unit_time_sec:    0
stat::peak_rss_mb:              164
```
</details>

### fuzz_diff
- ✅ Completed without crashes
- Corpus: 1183 → 1389 files
- Artifacts: 1 files in `fuzz/artifacts/fuzz_diff`

<details><summary>Final libFuzzer stats (tail)</summary>

```
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
Rust(worker): Unknown TYPE 0x08
#1877724	DONE   cov: 78 ft: 325 corp: 108/21Kb lim: 4096 exec/s: 60571 rss: 312Mb
###### Recommended dictionary. ######
"\000\000" # Uses: 22152
"\377\377" # Uses: 13562
"\377\377\377\377\377\377\377\377" # Uses: 4620
"\000\000\000\000" # Uses: 379
###### End of recommended dictionary. ######
Done 1877724 runs in 31 second(s)
stat::number_of_executed_units: 1877724
stat::average_exec_per_sec:     60571
stat::new_units_added:          206
stat::slowest_unit_time_sec:    0
stat::peak_rss_mb:              312
```
</details>
