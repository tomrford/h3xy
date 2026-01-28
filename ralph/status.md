Status: running

Memory:
- /AM (address multiply) and /AO (address offset) are NOT HexView options - testgen invented them.
- /CSE is not a separate option; exclude ranges use ;/ syntax: /CS0:@addr;/exclude_range
- HexView doesn't accept quoted ranges; h3xy strips quotes but HexView doesn't.
- Checksum byte order: algorithm name dictates output format (ByteSumLe=LE, ByteSumBe=BE); /CSR inverts.
- Checksum target exclusion: when target is Address or OverwriteEnd, exclude target bytes from calculation.
- Checksum gap handling: without forced range, only actual data bytes are checksummed (no 0xFF gap fill).
- Testgen naming mismatch: gen_crc32→/CS0=ByteSumBe, gen_crc16→/CS1=ByteSumLe, gen_bytesum→/CS2=WordSumBe.
- /SWAPWORD//SWAPLONG: HexView swaps complete chunks only; odd-length trailing bytes unchanged.
- HexView /SWAPWORD BUGS (known limitations):
  - Fails when input has odd-length Intel HEX records
  - Fails when input has Extended Linear Address records (type 04) for addresses < 0x10000
  - Fails when input has many gaps (>2 scattered segments)
  - h3xy handles these correctly; tests may need adjustment to match HexView rejection behavior
- Merge offset syntax: use semicolon (;) not colon (:) → /MO:file;offset (fixed in testgen.py)

Notes:
- Fresh run with updated prompt.md (100% parity goal, code quality focus)
- Investigate all failure categories: SWAP, checksums, large files, merge with offset
- When HexView rejects input, h3xy should reject it too (adjust tests to verify rejection parity)
- After fixing failures, look for code simplification opportunities

Progress:
- compare.sh updated to handle rejection parity: if BOTH tools reject input, count as PASS
- Analyzed 90 failures: 67 exit code 1 (mismatch), 19 exit code 2, 4 timeout
- SWAP operations: 37 failures - biggest category
- CHECKSUM operations: 25 failures
- Binary/S-Record output: 10 failures
- h3xy's SWAPWORD implementation is correct (verified locally)
- Issue may be HexView producing different output format (not algorithmic difference)
- Extended Linear Address records (type 04) for low addresses may cause HexView issues

Analysis:
- Most affected inputs: misaligned.hex (8), scattered.hex (7), large_sparse.hex (7)
- These match HexView's documented limitations with scattered/odd-length segments
- Without HexView access, cannot determine exact output differences
- h3xy's binary output concatenates segments in order (per manual), may differ from HexView

Fixed:
- CHECKSUM PERFORMANCE BUG (src/ops/checksum.rs): collect_data_for_checksum was iterating over
  entire address range even when no forced range was specified. For files with large address gaps
  (e.g., 0x0F00 to 0xFFFF_FF00), this caused iteration over ~4GB addresses and timeout.
  Fix: Split into two code paths - with forced_range iterates all addresses, without forced_range
  iterates only over actual data bytes from BTreeMap.
- This fixes 2 of 4 timeout failures (max_addr.hex + /CS operations)
- FILL RANGE OUTPUT ORDER (src/ops/filter.rs): fill_ranges was using prepend_segment, causing
  fill data to appear FIRST in /XN binary output. HexView manual says "order of appearance"
  which means original data first, then fill.
  Fix: Changed fill_ranges to compute actual gaps and fill only those, then append (not prepend).
  This gives correct order: original data segments first, fill segments last.

Remaining timeout issues:
- fuzz_flash_config_002 and fuzz_rand_008_000: These create files with /FR at low addresses
  followed by /FA, causing massive gap fill (~134MB for flash_config). This is pathological
  fuzz test behavior, not a real performance bug - actually filling that much memory IS slow.
- Consider: add max fill size limit, or filter out pathological fuzz tests

CS2 checksum failures (exit code 2):
- Pattern: /CS2:@address where address is outside existing data range
- Examples: scattered.hex (data ends ~0x4C3), /CS2:@0xFFC writes at 0xFFC
- h3xy succeeds (creates new segment for checksum), HexView likely rejects
- POSSIBLE FIX: Reject checksum target address if outside [min_address, max_address]
- Need HexView validation to confirm this is the behavioral difference

Current iteration:
- Fixed validation/src/h3xy_validation/intel_hex.py to use correct Extended Address record types:
  - HexView auto-mode: 16 bits = no extended, 17-20 bits = type 02 (Segment), 21+ bits = type 04 (Linear)
  - Testgen was always using type 04; now matches HexView behavior
  - This fixes format mismatch between h3xy output and test inputs
- All 285 unit/integration tests pass

Remaining failures (88 total from last validation run):
- SWAP ops: output format differences (scattered/misaligned/odd_length inputs)
- CS2 checksum (exit code 2): HexView rejects, h3xy accepts - behavior TBD
- /CR on large files: record format differences (now should match after intel_hex.py fix)
- Merge with negative offset: address underflow rejection
- Fuzz tests with /FA+/FR: pathological cases causing timeout
- Without HexView access, cannot verify exact diff for output mismatches

Next:
- Validation suite re-run needed to measure impact of intel_hex.py fix
- Focus on remaining exit code 2 failures (HexView rejection behavior)
