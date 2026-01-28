"""Hex file generator for creating varied test inputs."""

import random
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable

from .intel_hex import Segment, write_intel_hex


@dataclass
class GeneratorConfig:
    """Configuration for hex file generation."""

    seed: int = 42
    output_dir: Path = field(default_factory=lambda: Path("inputs"))

    # Size ranges
    min_segment_size: int = 16
    max_segment_size: int = 4096
    min_segments: int = 1
    max_segments: int = 5

    # Address characteristics
    typical_base_addresses: list[int] = field(
        default_factory=lambda: [
            0x0000,      # Start of memory
            0x0100,      # Common boot area end
            0x0800,      # STM32 flash start
            0x1000,      # Common page boundary
            0x8000,      # Common flash start (PIC, AVR)
            0x10000,     # 64K boundary
            0x08000000,  # STM32 absolute flash
        ]
    )

    # Alignment options
    common_alignments: list[int] = field(
        default_factory=lambda: [1, 2, 4, 8, 16, 32, 64, 256, 4096]
    )


class HexGenerator:
    """Generate varied hex files for testing."""

    def __init__(self, config: GeneratorConfig | None = None):
        self.config = config or GeneratorConfig()
        self.rng = random.Random(self.config.seed)
        self.config.output_dir.mkdir(parents=True, exist_ok=True)

    def reseed(self, seed: int) -> None:
        """Reset the random generator with a new seed."""
        self.rng = random.Random(seed)

    # ─────────────────────────────────────────────────────────────────────
    # Pattern generators
    # ─────────────────────────────────────────────────────────────────────

    def pattern_sequential(self, size: int, start: int = 0) -> bytes:
        """Sequential bytes: 00 01 02 03..."""
        return bytes((start + i) & 0xFF for i in range(size))

    def pattern_address_echo(self, start_addr: int, size: int) -> bytes:
        """Data equals address (low byte)."""
        return bytes((start_addr + i) & 0xFF for i in range(size))

    def pattern_constant(self, size: int, value: int = 0xFF) -> bytes:
        """Constant fill."""
        return bytes([value & 0xFF] * size)

    def pattern_alternating(self, size: int, a: int = 0xAA, b: int = 0x55) -> bytes:
        """Alternating bytes: AA 55 AA 55..."""
        return bytes([a, b] * (size // 2 + 1))[:size]

    def pattern_random(self, size: int) -> bytes:
        """Random bytes (seeded for reproducibility)."""
        return bytes(self.rng.randint(0, 255) for _ in range(size))

    def pattern_words(self, size: int, big_endian: bool = True) -> bytes:
        """Recognizable 16-bit words: DEAD BEEF CAFE..."""
        words = [0xDEAD, 0xBEEF, 0xCAFE, 0xBABE, 0xF00D, 0xC0DE, 0xFACE, 0xFEED]
        data = []
        for i in range(size // 2 + 1):
            w = words[i % len(words)]
            if big_endian:
                data.extend([w >> 8, w & 0xFF])
            else:
                data.extend([w & 0xFF, w >> 8])
        return bytes(data[:size])

    def pattern_ramp(self, size: int, step: int = 1) -> bytes:
        """Ramping values with configurable step."""
        return bytes((i * step) & 0xFF for i in range(size))

    # ─────────────────────────────────────────────────────────────────────
    # Segment generation
    # ─────────────────────────────────────────────────────────────────────

    def make_segment(
        self,
        start_address: int,
        size: int,
        pattern_fn: Callable[[int], bytes] | None = None,
    ) -> Segment:
        """Create a segment with the given parameters."""
        if pattern_fn is None:
            pattern_fn = self.pattern_random
        data = pattern_fn(size)
        return Segment(start_address=start_address, data=data)

    def make_aligned_segment(
        self,
        base_address: int,
        size: int,
        alignment: int = 16,
        pattern_fn: Callable[[int], bytes] | None = None,
    ) -> Segment:
        """Create a segment aligned to the specified boundary."""
        aligned_addr = (base_address // alignment) * alignment
        return self.make_segment(aligned_addr, size, pattern_fn)

    # ─────────────────────────────────────────────────────────────────────
    # File generators - Standard shapes
    # ─────────────────────────────────────────────────────────────────────

    def gen_single_segment(
        self,
        name: str,
        start_address: int = 0x0000,
        size: int = 256,
        pattern_fn: Callable[[int], bytes] | None = None,
    ) -> Path:
        """Generate file with single contiguous segment."""
        segment = self.make_segment(start_address, size, pattern_fn)
        path = self.config.output_dir / f"{name}.hex"
        write_intel_hex(str(path), [segment])
        return path

    def gen_multi_segment(
        self,
        name: str,
        segments_spec: list[tuple[int, int]],  # [(addr, size), ...]
        pattern_fn: Callable[[int], bytes] | None = None,
    ) -> Path:
        """Generate file with multiple segments at specified addresses."""
        segments = [
            self.make_segment(addr, size, pattern_fn)
            for addr, size in segments_spec
        ]
        path = self.config.output_dir / f"{name}.hex"
        write_intel_hex(str(path), segments)
        return path

    def gen_gapped(
        self,
        name: str,
        start_address: int = 0x0000,
        segment_sizes: list[int] | None = None,
        gap_sizes: list[int] | None = None,
        pattern_fn: Callable[[int], bytes] | None = None,
    ) -> Path:
        """Generate file with gaps between segments."""
        if segment_sizes is None:
            segment_sizes = [64, 128, 64]
        if gap_sizes is None:
            gap_sizes = [32, 64]

        segments = []
        addr = start_address
        for i, seg_size in enumerate(segment_sizes):
            segments.append(self.make_segment(addr, seg_size, pattern_fn))
            addr += seg_size
            if i < len(gap_sizes):
                addr += gap_sizes[i]

        path = self.config.output_dir / f"{name}.hex"
        write_intel_hex(str(path), segments)
        return path

    # ─────────────────────────────────────────────────────────────────────
    # File generators - Edge cases
    # ─────────────────────────────────────────────────────────────────────

    def gen_tiny(self, name: str = "tiny") -> Path:
        """Generate minimal file (1 byte)."""
        return self.gen_single_segment(name, size=1)

    def gen_single_byte_segments(self, name: str = "single_bytes") -> Path:
        """Multiple single-byte segments with gaps."""
        specs = [(0x0000, 1), (0x0010, 1), (0x0020, 1), (0x0100, 1)]
        return self.gen_multi_segment(name, specs)

    def gen_max_address(self, name: str = "max_addr") -> Path:
        """Segment near maximum 32-bit address."""
        return self.gen_single_segment(name, start_address=0xFFFFFF00, size=64)

    def gen_64k_boundary(self, name: str = "boundary_64k") -> Path:
        """Segment crossing 64K boundary (tests extended addressing)."""
        return self.gen_single_segment(name, start_address=0x0000FF80, size=256)

    def gen_misaligned(self, name: str = "misaligned") -> Path:
        """Segment at non-aligned address."""
        return self.gen_single_segment(name, start_address=0x1001, size=100)

    def gen_odd_length(self, name: str = "odd_length") -> Path:
        """Segment with odd byte count (swap edge case)."""
        return self.gen_single_segment(name, start_address=0x1000, size=101)

    # ─────────────────────────────────────────────────────────────────────
    # File generators - Realistic firmware patterns
    # ─────────────────────────────────────────────────────────────────────

    def gen_bootloader_app(self, name: str = "boot_app") -> Path:
        """Simulated bootloader + application layout."""
        specs = [
            (0x0000, 512),    # Reset vectors / bootloader
            (0x1000, 2048),   # Application
        ]
        return self.gen_multi_segment(name, specs)

    def gen_flash_with_config(self, name: str = "flash_config") -> Path:
        """Flash with separate config region."""
        specs = [
            (0x08000000, 4096),   # Main flash (STM32-style)
            (0x0800F000, 256),    # Config area
        ]
        return self.gen_multi_segment(name, specs)

    def gen_scattered(self, name: str = "scattered") -> Path:
        """Many small scattered segments."""
        num_segments = self.rng.randint(5, 10)
        specs = []
        addr = 0
        for _ in range(num_segments):
            # Force even sizes to avoid HexView /SWAPWORD bug with odd-length records
            size = self.rng.randint(8, 64) * 2  # 16-128, always even
            specs.append((addr, size))
            addr += size + self.rng.randint(32, 256)  # Random gap
        return self.gen_multi_segment(name, specs)

    # ─────────────────────────────────────────────────────────────────────
    # File generators - Large files (stress testing)
    # ─────────────────────────────────────────────────────────────────────

    def gen_large_contiguous(self, name: str = "large_64k") -> Path:
        """Large contiguous segment (64KB)."""
        return self.gen_single_segment(name, start_address=0x0000, size=65536)

    def gen_large_256k(self, name: str = "large_256k") -> Path:
        """Very large contiguous segment (256KB)."""
        return self.gen_single_segment(name, start_address=0x0000, size=262144)

    def gen_large_1m(self, name: str = "large_1m") -> Path:
        """Stress test: 1MB contiguous segment."""
        return self.gen_single_segment(name, start_address=0x0000, size=1048576)

    def gen_large_multi_segment(self, name: str = "large_multi") -> Path:
        """Large file with multiple segments totaling ~512KB."""
        specs = [
            (0x00000000, 131072),  # 128KB
            (0x00040000, 131072),  # 128KB
            (0x00080000, 131072),  # 128KB
            (0x000C0000, 131072),  # 128KB
        ]
        return self.gen_multi_segment(name, specs)

    def gen_large_sparse(self, name: str = "large_sparse") -> Path:
        """Large address space with sparse data."""
        specs = [
            (0x00000000, 4096),
            (0x00100000, 4096),
            (0x00200000, 4096),
            (0x00300000, 4096),
        ]
        return self.gen_multi_segment(name, specs)

    # ─────────────────────────────────────────────────────────────────────
    # File generators - Pattern variations
    # ─────────────────────────────────────────────────────────────────────

    def gen_sequential(self, name: str = "sequential") -> Path:
        """Sequential pattern for easy visual debugging."""
        return self.gen_single_segment(
            name, size=256, pattern_fn=self.pattern_sequential
        )

    def gen_constant_ff(self, name: str = "const_ff") -> Path:
        """All 0xFF (common erased flash)."""
        return self.gen_single_segment(
            name, size=256, pattern_fn=lambda s: self.pattern_constant(s, 0xFF)
        )

    def gen_constant_00(self, name: str = "const_00") -> Path:
        """All 0x00."""
        return self.gen_single_segment(
            name, size=256, pattern_fn=lambda s: self.pattern_constant(s, 0x00)
        )

    def gen_alternating(self, name: str = "alternating") -> Path:
        """Alternating pattern."""
        return self.gen_single_segment(
            name, size=256, pattern_fn=self.pattern_alternating
        )

    def gen_recognizable_words(self, name: str = "words") -> Path:
        """Recognizable word pattern."""
        return self.gen_single_segment(
            name, size=256, pattern_fn=self.pattern_words
        )

    # ─────────────────────────────────────────────────────────────────────
    # Batch generation
    # ─────────────────────────────────────────────────────────────────────

    def gen_standard_suite(self) -> dict[str, Path]:
        """Generate a standard suite of test files."""
        files = {}

        # Basic shapes
        files["single_small"] = self.gen_single_segment("single_small", size=64)
        files["single_medium"] = self.gen_single_segment("single_medium", size=512)
        files["single_large"] = self.gen_single_segment("single_large", size=4096)
        files["multi_basic"] = self.gen_gapped("multi_basic")
        files["scattered"] = self.gen_scattered("scattered")

        # Edge cases
        files["tiny"] = self.gen_tiny()
        files["single_bytes"] = self.gen_single_byte_segments()
        files["max_addr"] = self.gen_max_address()
        files["boundary_64k"] = self.gen_64k_boundary()
        files["misaligned"] = self.gen_misaligned()
        files["odd_length"] = self.gen_odd_length()

        # Realistic patterns
        files["boot_app"] = self.gen_bootloader_app()
        files["flash_config"] = self.gen_flash_with_config()

        # Large files (stress testing)
        files["large_64k"] = self.gen_large_contiguous()
        files["large_256k"] = self.gen_large_256k()
        files["large_multi"] = self.gen_large_multi_segment()
        files["large_sparse"] = self.gen_large_sparse()

        # Data patterns
        files["sequential"] = self.gen_sequential()
        files["const_ff"] = self.gen_constant_ff()
        files["const_00"] = self.gen_constant_00()
        files["alternating"] = self.gen_alternating()
        files["words"] = self.gen_recognizable_words()

        return files

    def gen_random_file(self, name: str) -> Path:
        """Generate a random hex file for fuzzing."""
        num_segments = self.rng.randint(
            self.config.min_segments, self.config.max_segments
        )

        # Pick a base address style
        base = self.rng.choice(self.config.typical_base_addresses)
        if self.rng.random() < 0.3:
            # Sometimes add random offset
            base += self.rng.randint(0, 0x1000)

        # Pick alignment (or none)
        alignment = self.rng.choice([1, 1, 1, 4, 16, 256])

        specs = []
        addr = base
        for _ in range(num_segments):
            # Force even sizes to avoid HexView /SWAPWORD bug with odd-length records
            min_half = self.config.min_segment_size // 2
            max_half = self.config.max_segment_size // 2
            size = self.rng.randint(min_half, max_half) * 2  # Always even

            # Align address
            if alignment > 1:
                addr = ((addr + alignment - 1) // alignment) * alignment

            specs.append((addr, size))
            addr += size + self.rng.randint(0, 512)  # Gap

        # Pick pattern
        patterns = [
            self.pattern_random,
            self.pattern_sequential,
            self.pattern_alternating,
            self.pattern_words,
            lambda s: self.pattern_constant(s, 0xFF),
        ]
        pattern_fn = self.rng.choice(patterns)

        return self.gen_multi_segment(name, specs, pattern_fn)

    def gen_random_suite(self, count: int = 20, prefix: str = "rand") -> dict[str, Path]:
        """Generate a suite of random test files."""
        files = {}
        for i in range(count):
            name = f"{prefix}_{i:03d}"
            files[name] = self.gen_random_file(name)
        return files

    # ─────────────────────────────────────────────────────────────────────
    # Merge pair generation
    # ─────────────────────────────────────────────────────────────────────

    def gen_merge_pair_overlapping(
        self, name_base: str = "merge_overlap"
    ) -> tuple[Path, Path]:
        """Generate two files with overlapping address ranges."""
        # File A: 0x1000-0x1FFF
        path_a = self.gen_single_segment(
            f"{name_base}_a",
            start_address=0x1000,
            size=0x1000,
            pattern_fn=lambda s: self.pattern_constant(s, 0xAA),
        )
        # File B: 0x1800-0x27FF (overlaps with A)
        path_b = self.gen_single_segment(
            f"{name_base}_b",
            start_address=0x1800,
            size=0x1000,
            pattern_fn=lambda s: self.pattern_constant(s, 0xBB),
        )
        return path_a, path_b

    def gen_merge_pair_adjacent(
        self, name_base: str = "merge_adjacent"
    ) -> tuple[Path, Path]:
        """Generate two files with adjacent (non-overlapping) ranges."""
        path_a = self.gen_single_segment(
            f"{name_base}_a",
            start_address=0x1000,
            size=0x800,
            pattern_fn=self.pattern_sequential,
        )
        path_b = self.gen_single_segment(
            f"{name_base}_b",
            start_address=0x1800,
            size=0x800,
            pattern_fn=lambda s: self.pattern_sequential(s, 0x80),
        )
        return path_a, path_b

    def gen_merge_pair_disjoint(
        self, name_base: str = "merge_disjoint"
    ) -> tuple[Path, Path]:
        """Generate two files with completely separate ranges."""
        path_a = self.gen_single_segment(
            f"{name_base}_a", start_address=0x0000, size=0x100
        )
        path_b = self.gen_single_segment(
            f"{name_base}_b", start_address=0x8000, size=0x100
        )
        return path_a, path_b
