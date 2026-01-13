"""Test case generator for h3xy validation.

Generates test cases combining input files with operations.
Uses HexView CLI syntax for compatibility with compare.sh.
"""

import random
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable


@dataclass
class TestCase:
    """A single test case to run."""

    name: str
    input_file: str  # Relative to inputs dir
    args: list[str]  # HexView-style arguments (excluding input and -o)
    output_name: str  # Output filename (without path)
    merge_file: str | None = None  # Optional second file for merge ops
    description: str = ""

    def to_args(self, inputs_dir: Path, outputs_dir: Path) -> list[str]:
        """Convert to full argument list for compare.sh."""
        args = [str(inputs_dir / self.input_file)]
        args.extend(self.args)

        # Handle merge file paths
        if self.merge_file:
            for i, arg in enumerate(args):
                if "{merge_file}" in arg:
                    args[i] = arg.format(merge_file=str(inputs_dir / self.merge_file))

        args.extend(["-o", str(outputs_dir / self.output_name)])
        return args

    def to_test_line(self) -> str:
        """Format as test file line for batch-compare.sh."""
        args_str = " ".join(self.args)
        return f"{self.name}: {self.input_file} {args_str} -o {self.output_name}"


@dataclass
class TestGeneratorConfig:
    """Configuration for test case generation."""

    seed: int = 42
    inputs_dir: Path = field(default_factory=lambda: Path("inputs"))
    outputs_dir: Path = field(default_factory=lambda: Path("outputs"))


class TestCaseGenerator:
    """Generate test cases for h3xy validation."""

    def __init__(self, config: TestGeneratorConfig | None = None):
        self.config = config or TestGeneratorConfig()
        self.rng = random.Random(self.config.seed)

    def reseed(self, seed: int) -> None:
        """Reset the random generator with a new seed."""
        self.rng = random.Random(seed)

    # ─────────────────────────────────────────────────────────────────────
    # Helper: Format values
    # ─────────────────────────────────────────────────────────────────────

    @staticmethod
    def fmt_addr(addr: int) -> str:
        """Format address as hex."""
        return f"0x{addr:X}"

    @staticmethod
    def fmt_range_se(start: int, end: int) -> str:
        """Format start-end range."""
        return f"0x{start:X}-0x{end:X}"

    @staticmethod
    def fmt_range_sl(start: int, length: int) -> str:
        """Format start,length range."""
        return f"0x{start:X},0x{length:X}"

    # ─────────────────────────────────────────────────────────────────────
    # Passthrough tests (baseline)
    # ─────────────────────────────────────────────────────────────────────

    def gen_passthrough(self, input_file: str) -> TestCase:
        """Simple passthrough - no operations."""
        name = f"passthrough_{Path(input_file).stem}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[],
            output_name=f"{name}.hex",
            description="Passthrough (read and write, no ops)",
        )

    # ─────────────────────────────────────────────────────────────────────
    # Address Range Filter (/AR)
    # ─────────────────────────────────────────────────────────────────────

    def gen_address_range(
        self, input_file: str, start: int, end: int, suffix: str = ""
    ) -> TestCase:
        """Address range filter."""
        stem = Path(input_file).stem
        name = f"ar_{stem}_{start:x}_{end:x}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[f"/AR:'{self.fmt_range_se(start, end)}'"],
            output_name=f"{name}.hex",
            description=f"Filter to range {start:#x}-{end:#x}",
        )

    def gen_address_range_variants(self, input_file: str) -> list[TestCase]:
        """Generate various address range filter tests."""
        tests = []

        # Common ranges
        ranges = [
            (0x0000, 0x00FF, "_first256"),
            (0x0000, 0x007F, "_first128"),
            (0x0080, 0x00FF, "_second128"),
            (0x0000, 0x0FFF, "_first4k"),
            (0x1000, 0x1FFF, "_second4k"),
        ]

        for start, end, suffix in ranges:
            tests.append(self.gen_address_range(input_file, start, end, suffix))

        return tests

    # ─────────────────────────────────────────────────────────────────────
    # Cut Range (/CR)
    # ─────────────────────────────────────────────────────────────────────

    def gen_cut_range(
        self, input_file: str, start: int, end: int, suffix: str = ""
    ) -> TestCase:
        """Cut (remove) range."""
        stem = Path(input_file).stem
        name = f"cr_{stem}_{start:x}_{end:x}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[f"/CR:'{self.fmt_range_se(start, end)}'"],
            output_name=f"{name}.hex",
            description=f"Cut range {start:#x}-{end:#x}",
        )

    def gen_cut_range_variants(self, input_file: str) -> list[TestCase]:
        """Generate various cut range tests."""
        tests = []

        cuts = [
            (0x0000, 0x000F, "_start"),      # Cut from start
            (0x00F0, 0x00FF, "_end"),        # Cut from end
            (0x0040, 0x007F, "_middle"),     # Cut from middle
            (0x0020, 0x005F, "_chunk"),      # Arbitrary chunk
        ]

        for start, end, suffix in cuts:
            tests.append(self.gen_cut_range(input_file, start, end, suffix))

        return tests

    # ─────────────────────────────────────────────────────────────────────
    # Fill Range (/FR + /FP)
    # ─────────────────────────────────────────────────────────────────────

    def gen_fill_range(
        self,
        input_file: str,
        start: int,
        length: int,
        pattern: str = "FF",
        suffix: str = "",
    ) -> TestCase:
        """Fill range with pattern."""
        stem = Path(input_file).stem
        name = f"fr_{stem}_{start:x}_{length:x}_{pattern}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[
                f"/FR:'{self.fmt_range_sl(start, length)}'",
                f"/FP:{pattern}",
            ],
            output_name=f"{name}.hex",
            description=f"Fill {length:#x} bytes at {start:#x} with {pattern}",
        )

    def gen_fill_range_variants(self, input_file: str) -> list[TestCase]:
        """Generate various fill range tests."""
        tests = []

        fills = [
            (0x0100, 0x0010, "00", "_zeros"),
            (0x0100, 0x0010, "FF", "_ones"),
            (0x0100, 0x0010, "AA", "_aa"),
            (0x0100, 0x0100, "DEADBEEF", "_pattern"),
            (0x2000, 0x0080, "55", "_gap"),  # Fill in gap area
        ]

        for start, length, pattern, suffix in fills:
            tests.append(self.gen_fill_range(input_file, start, length, pattern, suffix))

        return tests

    # ─────────────────────────────────────────────────────────────────────
    # Fill All (/FA)
    # ─────────────────────────────────────────────────────────────────────

    def gen_fill_all(
        self, input_file: str, pattern: str = "FF", suffix: str = ""
    ) -> TestCase:
        """Fill all gaps."""
        stem = Path(input_file).stem
        name = f"fa_{stem}_{pattern}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=["/FA", f"/FP:{pattern}"],
            output_name=f"{name}.hex",
            description=f"Fill all gaps with {pattern}",
        )

    # ─────────────────────────────────────────────────────────────────────
    # Align (/AD, /AL)
    # ─────────────────────────────────────────────────────────────────────

    def gen_align_data(
        self, input_file: str, alignment: int, suffix: str = ""
    ) -> TestCase:
        """Align addresses."""
        stem = Path(input_file).stem
        name = f"ad_{stem}_{alignment}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[f"/AD:{alignment}"],
            output_name=f"{name}.hex",
            description=f"Align addresses to {alignment} bytes",
        )

    def gen_align_length(self, input_file: str, suffix: str = "") -> TestCase:
        """Align lengths."""
        stem = Path(input_file).stem
        name = f"al_{stem}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=["/AL"],
            output_name=f"{name}.hex",
            description="Align segment lengths",
        )

    def gen_align_variants(self, input_file: str) -> list[TestCase]:
        """Generate various alignment tests."""
        tests = []

        for alignment in [2, 4, 8, 16, 32, 64, 256]:
            tests.append(self.gen_align_data(input_file, alignment, f"_{alignment}"))

        tests.append(self.gen_align_length(input_file))

        return tests

    # ─────────────────────────────────────────────────────────────────────
    # Split Blocks (/SB)
    # ─────────────────────────────────────────────────────────────────────

    def gen_split_blocks(
        self, input_file: str, block_size: int, suffix: str = ""
    ) -> TestCase:
        """Split into blocks."""
        stem = Path(input_file).stem
        name = f"sb_{stem}_{block_size:x}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[f"/SB:{self.fmt_addr(block_size)}"],
            output_name=f"{name}.hex",
            description=f"Split into {block_size:#x}-byte blocks",
        )

    def gen_split_variants(self, input_file: str) -> list[TestCase]:
        """Generate various split tests."""
        tests = []

        for size in [0x10, 0x20, 0x40, 0x80, 0x100]:
            tests.append(self.gen_split_blocks(input_file, size, f"_{size:x}"))

        return tests

    # ─────────────────────────────────────────────────────────────────────
    # Byte Swap (/SWAPWORD, /SWAPLONG)
    # ─────────────────────────────────────────────────────────────────────

    def gen_swap_word(self, input_file: str, suffix: str = "") -> TestCase:
        """Swap bytes in words."""
        stem = Path(input_file).stem
        name = f"swapword_{stem}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=["/SWAPWORD"],
            output_name=f"{name}.hex",
            description="Swap bytes in 16-bit words",
        )

    def gen_swap_long(self, input_file: str, suffix: str = "") -> TestCase:
        """Swap bytes in dwords."""
        stem = Path(input_file).stem
        name = f"swaplong_{stem}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=["/SWAPLONG"],
            output_name=f"{name}.hex",
            description="Swap bytes in 32-bit dwords",
        )

    # ─────────────────────────────────────────────────────────────────────
    # Checksum (/CS0, /CS1, /CS2, etc.)
    # ─────────────────────────────────────────────────────────────────────

    def gen_crc32(
        self, input_file: str, target_addr: int, suffix: str = ""
    ) -> TestCase:
        """CRC-32 at address."""
        stem = Path(input_file).stem
        name = f"crc32_{stem}_{target_addr:x}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[f"/CS0:@{self.fmt_addr(target_addr)}"],
            output_name=f"{name}.hex",
            description=f"CRC-32 at {target_addr:#x}",
        )

    def gen_crc16(
        self, input_file: str, target_addr: int, suffix: str = ""
    ) -> TestCase:
        """CRC-16 at address."""
        stem = Path(input_file).stem
        name = f"crc16_{stem}_{target_addr:x}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[f"/CS1:@{self.fmt_addr(target_addr)}"],
            output_name=f"{name}.hex",
            description=f"CRC-16 at {target_addr:#x}",
        )

    def gen_bytesum(
        self, input_file: str, target_addr: int, suffix: str = ""
    ) -> TestCase:
        """Byte sum checksum at address."""
        stem = Path(input_file).stem
        name = f"bytesum_{stem}_{target_addr:x}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[f"/CS2:@{self.fmt_addr(target_addr)}"],
            output_name=f"{name}.hex",
            description=f"Byte sum at {target_addr:#x}",
        )

    def gen_crc_with_range(
        self,
        input_file: str,
        target_addr: int,
        range_start: int,
        range_end: int,
        crc_type: str = "/CS0",
        suffix: str = "",
    ) -> TestCase:
        """CRC over specific range."""
        stem = Path(input_file).stem
        name = f"crc_range_{stem}_{range_start:x}_{range_end:x}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[
                f"{crc_type}:@{self.fmt_addr(target_addr)}",
                f"/CSR:'{self.fmt_range_se(range_start, range_end)}'",
            ],
            output_name=f"{name}.hex",
            description=f"CRC over range {range_start:#x}-{range_end:#x}",
        )

    def gen_crc_with_exclude(
        self,
        input_file: str,
        target_addr: int,
        exclude_start: int,
        exclude_end: int,
        crc_type: str = "/CS0",
        suffix: str = "",
    ) -> TestCase:
        """CRC excluding specific range."""
        stem = Path(input_file).stem
        name = f"crc_excl_{stem}_{exclude_start:x}_{exclude_end:x}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[
                f"{crc_type}:@{self.fmt_addr(target_addr)}",
                f"/CSE:'{self.fmt_range_se(exclude_start, exclude_end)}'",
            ],
            output_name=f"{name}.hex",
            description=f"CRC excluding {exclude_start:#x}-{exclude_end:#x}",
        )

    def gen_checksum_variants(self, input_file: str) -> list[TestCase]:
        """Generate various checksum tests."""
        tests = []

        # CRC at various locations
        for addr in [0x00FC, 0x01FC, 0x0FFC]:
            tests.append(self.gen_crc32(input_file, addr, f"_{addr:x}"))
            tests.append(self.gen_crc16(input_file, addr, f"_{addr:x}"))
            tests.append(self.gen_bytesum(input_file, addr, f"_{addr:x}"))

        # CRC with range restriction
        tests.append(self.gen_crc_with_range(
            input_file, 0x00FC, 0x0000, 0x00F8, "/CS0", "_range"
        ))
        tests.append(self.gen_crc_with_range(
            input_file, 0x01FC, 0x0000, 0x01F8, "/CS1", "_range"
        ))

        # CRC with exclusion
        tests.append(self.gen_crc_with_exclude(
            input_file, 0x00FC, 0x0080, 0x00BF, "/CS0", "_excl"
        ))

        return tests

    # ─────────────────────────────────────────────────────────────────────
    # Output format variants (/XI:N, /XS, /XN)
    # ─────────────────────────────────────────────────────────────────────

    def gen_output_format_intel(
        self, input_file: str, bytes_per_line: int, suffix: str = ""
    ) -> TestCase:
        """Intel HEX output with specific bytes per line."""
        stem = Path(input_file).stem
        name = f"xi_{stem}_{bytes_per_line}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[f"/XI:{bytes_per_line}"],
            output_name=f"{name}.hex",
            description=f"Intel HEX with {bytes_per_line} bytes/line",
        )

    def gen_output_format_srec(
        self, input_file: str, bytes_per_line: int = 16, suffix: str = ""
    ) -> TestCase:
        """S-Record output."""
        stem = Path(input_file).stem
        name = f"xs_{stem}_{bytes_per_line}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[f"/XS:{bytes_per_line}"],
            output_name=f"{name}.mot",
            description=f"S-Record with {bytes_per_line} bytes/line",
        )

    def gen_output_format_binary(self, input_file: str, suffix: str = "") -> TestCase:
        """Binary output."""
        stem = Path(input_file).stem
        name = f"xn_{stem}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=["/XN"],
            output_name=f"{name}.bin",
            description="Binary output",
        )

    def gen_output_format_variants(self, input_file: str) -> list[TestCase]:
        """Generate output format variants."""
        tests = []

        # Intel HEX with different bytes per line
        for bpl in [8, 16, 32, 64]:
            tests.append(self.gen_output_format_intel(input_file, bpl))

        # S-Record variants
        for bpl in [16, 32]:
            tests.append(self.gen_output_format_srec(input_file, bpl))

        # Binary
        tests.append(self.gen_output_format_binary(input_file))

        return tests

    # ─────────────────────────────────────────────────────────────────────
    # Merge operations (/MT, /MO) with offset variants
    # ─────────────────────────────────────────────────────────────────────

    def gen_merge_overwrite(
        self, input_file: str, merge_file: str, suffix: str = ""
    ) -> TestCase:
        """Merge with overwrite."""
        stem = Path(input_file).stem
        name = f"mo_{stem}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=["/MO:{merge_file}"],
            output_name=f"{name}.hex",
            merge_file=merge_file,
            description=f"Merge {merge_file} (overwrite mode)",
        )

    def gen_merge_preserve(
        self, input_file: str, merge_file: str, suffix: str = ""
    ) -> TestCase:
        """Merge with preserve (transparent)."""
        stem = Path(input_file).stem
        name = f"mt_{stem}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=["/MT:{merge_file}"],
            output_name=f"{name}.hex",
            merge_file=merge_file,
            description=f"Merge {merge_file} (preserve mode)",
        )

    def gen_merge_with_offset(
        self,
        input_file: str,
        merge_file: str,
        offset: int,
        overwrite: bool = True,
        suffix: str = "",
    ) -> TestCase:
        """Merge with address offset."""
        stem = Path(input_file).stem
        mode = "mo" if overwrite else "mt"
        name = f"{mode}_offset_{stem}_{offset:x}{suffix}"
        op = "/MO" if overwrite else "/MT"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[f"{op}:{{merge_file}}:{self.fmt_addr(offset)}"],
            output_name=f"{name}.hex",
            merge_file=merge_file,
            description=f"Merge with offset {offset:#x}",
        )

    def gen_merge_with_negative_offset(
        self,
        input_file: str,
        merge_file: str,
        offset: int,
        overwrite: bool = True,
        suffix: str = "",
    ) -> TestCase:
        """Merge with negative address offset."""
        stem = Path(input_file).stem
        mode = "mo" if overwrite else "mt"
        name = f"{mode}_negoff_{stem}_{offset:x}{suffix}"
        op = "/MO" if overwrite else "/MT"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[f"{op}:{{merge_file}}:-{self.fmt_addr(offset)}"],
            output_name=f"{name}.hex",
            merge_file=merge_file,
            description=f"Merge with negative offset -{offset:#x}",
        )

    # ─────────────────────────────────────────────────────────────────────
    # Address scaling (/AM, /AD)
    # ─────────────────────────────────────────────────────────────────────

    def gen_address_multiply(
        self, input_file: str, factor: int, suffix: str = ""
    ) -> TestCase:
        """Multiply addresses by factor."""
        stem = Path(input_file).stem
        name = f"am_{stem}_{factor}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[f"/AM:{factor}"],
            output_name=f"{name}.hex",
            description=f"Multiply addresses by {factor}",
        )

    def gen_address_divide(
        self, input_file: str, divisor: int, suffix: str = ""
    ) -> TestCase:
        """Divide addresses by divisor."""
        stem = Path(input_file).stem
        name = f"ad_{stem}_{divisor}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[f"/AD:{divisor}"],
            output_name=f"{name}.hex",
            description=f"Divide addresses by {divisor}",
        )

    def gen_address_offset(
        self, input_file: str, offset: int, suffix: str = ""
    ) -> TestCase:
        """Add offset to all addresses."""
        stem = Path(input_file).stem
        name = f"ao_{stem}_{offset:x}{suffix}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[f"/AO:{self.fmt_addr(offset)}"],
            output_name=f"{name}.hex",
            description=f"Add offset {offset:#x} to addresses",
        )

    def gen_address_scale_variants(self, input_file: str) -> list[TestCase]:
        """Generate address scaling variants."""
        tests = []

        # Multiply
        for factor in [2, 4]:
            tests.append(self.gen_address_multiply(input_file, factor))

        # Offset
        for offset in [0x1000, 0x8000, 0x10000]:
            tests.append(self.gen_address_offset(input_file, offset))

        return tests

    # ─────────────────────────────────────────────────────────────────────
    # Combined operations
    # ─────────────────────────────────────────────────────────────────────

    def gen_combined_ar_fa(
        self, input_file: str, start: int, end: int, pattern: str = "FF"
    ) -> TestCase:
        """Filter range then fill gaps."""
        stem = Path(input_file).stem
        name = f"ar_fa_{stem}_{start:x}_{end:x}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[
                f"/AR:'{self.fmt_range_se(start, end)}'",
                "/FA",
                f"/FP:{pattern}",
            ],
            output_name=f"{name}.hex",
            description=f"Filter to {start:#x}-{end:#x} then fill gaps",
        )

    def gen_combined_fa_crc(
        self, input_file: str, crc_addr: int, pattern: str = "FF"
    ) -> TestCase:
        """Fill gaps then add CRC."""
        stem = Path(input_file).stem
        name = f"fa_crc_{stem}_{crc_addr:x}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[
                "/FA",
                f"/FP:{pattern}",
                f"/CS0:@{self.fmt_addr(crc_addr)}",
            ],
            output_name=f"{name}.hex",
            description=f"Fill gaps then CRC-32 at {crc_addr:#x}",
        )

    def gen_combined_ad_al(
        self, input_file: str, alignment: int
    ) -> TestCase:
        """Align addresses and lengths."""
        stem = Path(input_file).stem
        name = f"ad_al_{stem}_{alignment}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[f"/AD:{alignment}", "/AL"],
            output_name=f"{name}.hex",
            description=f"Align addresses and lengths to {alignment}",
        )

    def gen_combined_ar_cr(
        self, input_file: str, ar_start: int, ar_end: int, cr_start: int, cr_end: int
    ) -> TestCase:
        """Filter range then cut within it."""
        stem = Path(input_file).stem
        name = f"ar_cr_{stem}_{ar_start:x}_{cr_start:x}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[
                f"/AR:'{self.fmt_range_se(ar_start, ar_end)}'",
                f"/CR:'{self.fmt_range_se(cr_start, cr_end)}'",
            ],
            output_name=f"{name}.hex",
            description=f"Filter to {ar_start:#x}-{ar_end:#x} then cut {cr_start:#x}-{cr_end:#x}",
        )

    def gen_combined_fa_swap_crc(
        self, input_file: str, crc_addr: int, swap_mode: str = "/SWAPWORD"
    ) -> TestCase:
        """Fill gaps, swap bytes, add CRC."""
        stem = Path(input_file).stem
        name = f"fa_swap_crc_{stem}_{crc_addr:x}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[
                "/FA",
                "/FP:FF",
                swap_mode,
                f"/CS0:@{self.fmt_addr(crc_addr)}",
            ],
            output_name=f"{name}.hex",
            description="Fill gaps, swap bytes, add CRC-32",
        )

    def gen_combined_ar_fa_sb(
        self, input_file: str, start: int, end: int, split_size: int
    ) -> TestCase:
        """Filter, fill, split."""
        stem = Path(input_file).stem
        name = f"ar_fa_sb_{stem}_{start:x}_{split_size:x}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[
                f"/AR:'{self.fmt_range_se(start, end)}'",
                "/FA",
                "/FP:FF",
                f"/SB:{self.fmt_addr(split_size)}",
            ],
            output_name=f"{name}.hex",
            description=f"Filter, fill gaps, split to {split_size:#x}",
        )

    def gen_combined_fr_crc(
        self, input_file: str, fill_start: int, fill_len: int, crc_addr: int
    ) -> TestCase:
        """Fill specific region then add CRC."""
        stem = Path(input_file).stem
        name = f"fr_crc_{stem}_{fill_start:x}_{crc_addr:x}"
        return TestCase(
            name=name,
            input_file=input_file,
            args=[
                f"/FR:'{self.fmt_range_sl(fill_start, fill_len)}'",
                "/FP:00",
                f"/CS0:@{self.fmt_addr(crc_addr)}",
            ],
            output_name=f"{name}.hex",
            description=f"Fill {fill_start:#x}+{fill_len:#x}, then CRC at {crc_addr:#x}",
        )

    def gen_combined_variants(self, input_file: str) -> list[TestCase]:
        """Generate various combined operation tests."""
        tests = []

        # AR + FA
        tests.append(self.gen_combined_ar_fa(input_file, 0x0000, 0x0FFF))
        tests.append(self.gen_combined_ar_fa(input_file, 0x0000, 0x07FF, "00"))

        # FA + CRC
        tests.append(self.gen_combined_fa_crc(input_file, 0x0FFC))
        tests.append(self.gen_combined_fa_crc(input_file, 0x1FFC, "00"))

        # AD + AL
        tests.append(self.gen_combined_ad_al(input_file, 4))
        tests.append(self.gen_combined_ad_al(input_file, 16))

        # AR + CR
        tests.append(self.gen_combined_ar_cr(input_file, 0x0000, 0x0FFF, 0x0400, 0x07FF))

        # FA + SWAP + CRC
        tests.append(self.gen_combined_fa_swap_crc(input_file, 0x0FFC, "/SWAPWORD"))

        # AR + FA + SB
        tests.append(self.gen_combined_ar_fa_sb(input_file, 0x0000, 0x0FFF, 0x100))

        # FR + CRC
        tests.append(self.gen_combined_fr_crc(input_file, 0x0F00, 0xF8, 0x0FFC))

        return tests

    # ─────────────────────────────────────────────────────────────────────
    # Random test generation
    # ─────────────────────────────────────────────────────────────────────

    def gen_random_test(self, input_file: str, idx: int) -> TestCase:
        """Generate a random test case for fuzzing."""
        stem = Path(input_file).stem
        name = f"fuzz_{stem}_{idx:03d}"

        # Pick random operations
        ops = []
        output_ext = "hex"

        # Maybe filter range
        if self.rng.random() < 0.3:
            start = self.rng.randint(0, 0x0800)
            length = self.rng.randint(0x100, 0x1000)
            ops.append(f"/AR:'{self.fmt_range_sl(start, length)}'")

        # Maybe cut range
        if self.rng.random() < 0.2:
            start = self.rng.randint(0, 0x0400)
            length = self.rng.randint(0x10, 0x100)
            ops.append(f"/CR:'{self.fmt_range_sl(start, length)}'")

        # Maybe fill
        if self.rng.random() < 0.3:
            start = self.rng.randint(0, 0x0800)
            length = self.rng.randint(0x10, 0x200)
            pattern = self.rng.choice(["00", "FF", "AA", "55", "DEADBEEF"])
            ops.append(f"/FR:'{self.fmt_range_sl(start, length)}'")
            ops.append(f"/FP:{pattern}")

        # Maybe fill all
        if self.rng.random() < 0.2:
            pattern = self.rng.choice(["00", "FF"])
            ops.append("/FA")
            ops.append(f"/FP:{pattern}")

        # Maybe align
        if self.rng.random() < 0.3:
            alignment = self.rng.choice([2, 4, 8, 16, 32])
            ops.append(f"/AD:{alignment}")
            if self.rng.random() < 0.5:
                ops.append("/AL")

        # Maybe split
        if self.rng.random() < 0.2:
            size = self.rng.choice([0x20, 0x40, 0x80, 0x100])
            ops.append(f"/SB:{self.fmt_addr(size)}")

        # Maybe swap
        if self.rng.random() < 0.15:
            ops.append(self.rng.choice(["/SWAPWORD", "/SWAPLONG"]))

        # Maybe address offset
        if self.rng.random() < 0.15:
            offset = self.rng.choice([0x1000, 0x4000, 0x8000])
            ops.append(f"/AO:{self.fmt_addr(offset)}")

        # Maybe address multiply
        if self.rng.random() < 0.1:
            factor = self.rng.choice([2, 4])
            ops.append(f"/AM:{factor}")

        # Maybe CRC (at end, after other ops)
        if self.rng.random() < 0.25:
            addr = self.rng.choice([0x00FC, 0x01FC, 0x0FFC, 0x1FFC])
            crc_type = self.rng.choice(["/CS0", "/CS1", "/CS2"])
            ops.append(f"{crc_type}:@{self.fmt_addr(addr)}")

            # Maybe add range restriction
            if self.rng.random() < 0.3:
                range_end = addr - 4
                ops.append(f"/CSR:'{self.fmt_range_se(0, range_end)}'")

        # Maybe change output format
        if self.rng.random() < 0.2:
            fmt_choice = self.rng.choice(["xi8", "xi32", "xs", "xn"])
            if fmt_choice == "xi8":
                ops.append("/XI:8")
            elif fmt_choice == "xi32":
                ops.append("/XI:32")
            elif fmt_choice == "xs":
                ops.append("/XS:16")
                output_ext = "mot"
            elif fmt_choice == "xn":
                ops.append("/XN")
                output_ext = "bin"

        # Ensure at least one operation
        if not ops:
            ops.append(f"/AR:'{self.fmt_range_se(0, 0xFF)}'")

        return TestCase(
            name=name,
            input_file=input_file,
            args=ops,
            output_name=f"{name}.{output_ext}",
            description=f"Random fuzz test {idx}",
        )

    # ─────────────────────────────────────────────────────────────────────
    # Batch generation
    # ─────────────────────────────────────────────────────────────────────

    def gen_standard_suite(self, input_files: list[str]) -> list[TestCase]:
        """Generate a standard suite of tests for given input files."""
        tests = []

        for input_file in input_files:
            # Passthrough
            tests.append(self.gen_passthrough(input_file))

            # Address range variants
            tests.extend(self.gen_address_range_variants(input_file))

            # Cut variants
            tests.extend(self.gen_cut_range_variants(input_file))

            # Fill variants
            tests.extend(self.gen_fill_range_variants(input_file))

            # Fill all
            tests.append(self.gen_fill_all(input_file, "FF"))
            tests.append(self.gen_fill_all(input_file, "00", "_zeros"))

            # Align variants
            tests.extend(self.gen_align_variants(input_file))

            # Split variants
            tests.extend(self.gen_split_variants(input_file))

            # Swap (only for even-length files)
            tests.append(self.gen_swap_word(input_file))
            tests.append(self.gen_swap_long(input_file))

            # Checksum variants (CRC-32, CRC-16, bytesum, with range/exclude)
            tests.extend(self.gen_checksum_variants(input_file))

            # Output format variants (different bytes/line, S-Record, binary)
            tests.extend(self.gen_output_format_variants(input_file))

            # Address scaling variants (multiply, offset)
            tests.extend(self.gen_address_scale_variants(input_file))

            # Combined operations (multi-step pipelines)
            tests.extend(self.gen_combined_variants(input_file))

        return tests

    def gen_fuzz_suite(
        self, input_files: list[str], tests_per_file: int = 10
    ) -> list[TestCase]:
        """Generate random fuzz tests."""
        tests = []
        for input_file in input_files:
            for i in range(tests_per_file):
                tests.append(self.gen_random_test(input_file, i))
        return tests

    def gen_merge_suite(
        self, merge_pairs: list[tuple[str, str]]
    ) -> list[TestCase]:
        """Generate merge tests for file pairs."""
        tests = []
        for i, (file_a, file_b) in enumerate(merge_pairs):
            # Basic merge
            tests.append(self.gen_merge_overwrite(file_a, file_b, f"_{i}"))
            tests.append(self.gen_merge_preserve(file_a, file_b, f"_{i}"))

            # Merge with positive offsets
            for offset in [0x1000, 0x4000]:
                tests.append(self.gen_merge_with_offset(
                    file_a, file_b, offset, overwrite=True, suffix=f"_{i}_p{offset:x}"
                ))
                tests.append(self.gen_merge_with_offset(
                    file_a, file_b, offset, overwrite=False, suffix=f"_{i}_p{offset:x}"
                ))

            # Merge with negative offset
            tests.append(self.gen_merge_with_negative_offset(
                file_a, file_b, 0x800, overwrite=True, suffix=f"_{i}"
            ))

        return tests

    def write_test_file(self, tests: list[TestCase], path: Path) -> None:
        """Write tests to a batch-compare.sh compatible file."""
        with open(path, "w") as f:
            f.write("# Generated test cases for h3xy validation\n")
            f.write(f"# Count: {len(tests)}\n\n")
            for test in tests:
                if test.description:
                    f.write(f"# {test.description}\n")
                f.write(f"{test.to_test_line()}\n")
