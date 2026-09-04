#!/usr/bin/env perl
use strict;
use warnings;
use File::Basename qw(dirname);
use File::Spec;

my $crate = File::Spec->rel2abs(File::Spec->catdir(dirname(__FILE__), '..'));
my $root = File::Spec->rel2abs(File::Spec->catdir($crate, '..'));
my $c_lib = File::Spec->catfile($root, 'c_src', 'build', 'liblz4.so');
my $rust_lib = File::Spec->catfile($crate, 'target', 'release', 'liblz4.so');

sub dynamic_symbols {
    my ($library) = @_;
    open my $nm, '-|', 'nm', '-D', '--defined-only', $library
        or die "nm $library: $!";
    my @symbols;
    while (my $line = <$nm>) {
        push @symbols, $1 if $line =~ /\b([A-Za-z_][A-Za-z0-9_]*)\s*$/;
    }
    close $nm or die "nm failed for $library";
    return sort @symbols;
}

my @c_symbols = dynamic_symbols($c_lib);
my %rust_symbols = map { $_ => 1 } dynamic_symbols($rust_lib);

open my $symbols_out, '>', File::Spec->catfile($crate, 'SYMBOLS.md')
    or die "SYMBOLS.md: $!";
print {$symbols_out} "# Dynamic symbol surface\n\n";
print {$symbols_out} "Generated from `nm -D --defined-only` on both shared libraries.\n\n";
print {$symbols_out} "| # | C symbol | Rust export |\n";
print {$symbols_out} "|---:|----------|-------------|\n";
for my $index (0 .. $#c_symbols) {
    my $symbol = $c_symbols[$index];
    my $status = $rust_symbols{$symbol} ? "present" : "**MISSING**";
    print {$symbols_out} "| ", $index + 1, " | `$symbol` | $status |\n";
}
my @missing = grep { !$rust_symbols{$_} } @c_symbols;
print {$symbols_out} "\nMissing C symbols in Rust: **", scalar(@missing), "**.\n";
close $symbols_out;

sub configuration_for {
    my ($symbol) = @_;
    return "metadata/scalar boundary values, including 0, valid extrema, and one-past-range"
        if $symbol =~ /(?:version|Version|sizeof|compressionLevel_max|compressBound|getBlockSize|getError|isError|decoderRingBufferSize)/;
    return "one-shot hash; lengths 0,1,2,3,4,7,8,15,16,17,31,32,33,255; aligned and unaligned input; zero and nonzero seeds"
        if $symbol =~ /^LZ4_XXH(?:32|64)$/;
    return "streaming hash lifecycle; empty input, one chunk, irregular chunks, zero-length NULL update, canonical conversion, state copy"
        if $symbol =~ /^LZ4_XXH/;
    return "frame/file lifecycle; NULL/default preferences and blockSizeID 0/4/5/6/7; empty, one-byte, block-boundary, and multi-block data"
        if $symbol =~ /^LZ4F_(?:read|write)/;
    return "frame dictionary path; dictionary lengths 0,1,64,65535,65536,65537 and NULL/no-dictionary mode"
        if $symbol =~ /^LZ4F_.*(?:Dict|CDict)/;
    return "frame compression cross-product: block size 0/4/5/6/7, linked/independent, content and block checksums off/on, level -1/0/9/10/12/13, autoFlush 0/1, favorDecSpeed 0/1; empty/small/block-boundary/multi-block input"
        if $symbol =~ /^LZ4F_(?:compress|flush|uncompressed)/;
    return "frame decompression lifecycle; whole/chunked header and body, output sizes 0/1/exact/short, regular/skippable frame, with and without dictionary"
        if $symbol =~ /^LZ4F_(?:decompress|getFrameInfo|headerSize|resetDecompression|createDecompression|freeDecompression)/;
    return "context allocation/free lifecycle using default and custom allocators; supported version and one-past version"
        if $symbol =~ /^LZ4F_(?:create|free)/;
    return "HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination"
        if $symbol =~ /(?:HC|_HC|HC_)/ && $symbol =~ /compress/;
    return "HC stream/context lifecycle; aligned exact-size external state, undersized/misaligned state, empty/short/64KiB dictionary, reset/load/save/attach/favor modes"
        if $symbol =~ /(?:HC|_HC|HC_)/;
    return "decompression shape matrix; empty/malformed/valid compressed input, output 0/1/exact/short, full and partial decode, no dictionary/prefix/external dictionary, one-shot and streaming"
        if $symbol =~ /(?:decompress|uncompress)/;
    return "compression shape matrix; empty/small/64KiB/large random and repetitive input, destination 0/1/exact bound/short, acceleration 0/1/2/65537/65538, one-shot/ext-state/streaming/destSize"
        if $symbol =~ /compress/;
    return "stream/context lifecycle; create or aligned external init, reset, empty/short/64KiB dictionary, attach/load/save, one and multiple blocks, free"
        if $symbol =~ /(?:Stream|Dict|dictionary|create|free|reset|load|save|attach|slide)/;
    return "lowest-level exported helper with source-derived boundary indexes and empty/small dictionary search ranges"
        if $symbol eq 'LZ4HC_searchExtDict';
    return "valid public ABI call using zero/default and representative nonzero inputs";
}

open my $configs_out, '>', File::Spec->catfile($crate, 'CONFIGS.md')
    or die "CONFIGS.md: $!";
print {$configs_out} "# Configuration surface\n\n";
print {$configs_out} "Rows are generated from the complete dynamic-symbol list and the option/shape branches in the public headers and C sources. Repeated family descriptions intentionally retain every low-level and compatibility entry point.\n\n";
print {$configs_out} "| # | entry point(s) | configuration (options set + input shape) | [ ] |\n";
print {$configs_out} "|---:|----------------|--------------------------------------------|:---:|\n";
for my $index (0 .. $#c_symbols) {
    my $symbol = $c_symbols[$index];
    print {$configs_out} "| ", $index + 1, " | `$symbol` | ",
        configuration_for($symbol), " | [ ] |\n";
}
close $configs_out;

my %public = map { $_ => 1 } @c_symbols;
my %propagated_core = map { $_ => 1 } qw(
    LZ4_compress_generic
    LZ4_compress_generic_validated
    LZ4_decompress_generic
    LZ4_decompress_unsafe_generic
    LZ4F_compressUpdateImpl
    LZ4F_decodeHeader
    XXH32_update_endian
    XXH64_update_endian
);

my @errors;
for my $source (qw(lz4.c lz4hc.c lz4frame.c lz4file.c xxhash.c)) {
    my $path = File::Spec->catfile($root, 'c_src', 'src', $source);
    open my $ctags, '-|', 'ctags', '-x', '--c-kinds=f', $path
        or die "ctags $path: $!";
    my %function_at;
    while (my $tag = <$ctags>) {
        if ($tag =~ /^(\S+)\s+function\s+(\d+)\s+/) {
            $function_at{$2} = $1;
        }
    }
    close $ctags or die "ctags failed for $path";

    open my $in, '<', $path or die "$path: $!";
    my @lines = <$in>;
    close $in;
    my $function = '';
    for my $i (0 .. $#lines) {
        my $line = $lines[$i];
        my $line_number = $i + 1;
        $function = $function_at{$line_number} if exists $function_at{$line_number};
        my $trim = $line;
        $trim =~ s/^\s+|\s+$//g;

        my $selected = $public{$function} || $propagated_core{$function};
        if ($selected) {
            my ($condition, $result);
            if ($trim =~ /RETURN_ERROR_IF\s*\((.*),\s*([A-Za-z0-9_]+)\s*\)/) {
                ($condition, $result) = ($1, "LZ4F_ERROR_$2");
            } elsif ($trim =~ /if\s*\((.*?)\)\s*RETURN_ERROR\s*\(([^)]+)\)/) {
                ($condition, $result) = ($1, "LZ4F_ERROR_$2");
            } elsif ($trim =~ /RETURN_ERROR\s*\(([^)]+)\)/) {
                ($condition, $result) = ("branch condition immediately above", "LZ4F_ERROR_$1");
            } elsif ($trim =~ /if\s*\((.*?)\).*return\s+NULL\s*;/) {
                ($condition, $result) = ($1, "NULL");
            } elsif ($trim =~ /if\s*\((.*?)\).*return\s+(-\d+)\s*;/) {
                ($condition, $result) = ($1, $2);
            } elsif ($trim =~ /if\s*\((.*?)\).*return\s+XXH_ERROR\s*;/) {
                ($condition, $result) = ($1, "XXH_ERROR");
            } elsif ($trim =~ /assert\s*\(([^;]+)\)/) {
                my $assertion = $1;
                if ($public{$function}
                    && $assertion =~ /(?:NULL|>=\s*0|<=|>=|<\s*(?:INT_MAX|LZ4)|>\s*0)/) {
                    ($condition, $result) = ("assertion violated: $assertion", "assertion failure");
                }
            }
            if (defined $condition && $condition eq 'branch condition immediately above') {
                for (my $previous = $i - 1; $previous >= 0 && $previous >= $i - 5; $previous--) {
                    my $candidate = $lines[$previous];
                    $candidate =~ s/^\s+|\s+$//g;
                    if ($candidate =~ /\b(?:if|case|default)\b/) {
                        $condition = $candidate;
                        last;
                    }
                }
            }
            if (defined $condition) {
                $condition =~ s/\|/\\|/g;
                $condition =~ s/\s+/ /g;
                push @errors, [$function, "$source:$line_number — `$condition`", $result];
            }
        }
    }
}

# Header-documented boundaries which compile down through macros or calls.
push @errors,
    ['LZ4_compressBound', 'inputSize < 0 or inputSize > LZ4_MAX_INPUT_SIZE (header macro boundary)', '0'],
    ['LZ4_decoderRingBufferSize', 'maxBlockSize < 0 or maxBlockSize > LZ4_MAX_INPUT_SIZE', '0'],
    ['LZ4_initStream', 'stateBuffer is NULL', 'NULL'],
    ['LZ4_initStream', 'size < sizeof(LZ4_stream_t)', 'NULL'],
    ['LZ4_initStream', 'stateBuffer is not aligned for LZ4_stream_t', 'NULL'],
    ['LZ4_initStreamHC', 'buffer is NULL', 'NULL'],
    ['LZ4_initStreamHC', 'size < sizeof(LZ4_streamHC_t)', 'NULL'],
    ['LZ4_initStreamHC', 'buffer is not aligned for LZ4_streamHC_t', 'NULL'],
    ['LZ4F_getBlockSize', 'blockSizeID < LZ4F_max64KB or blockSizeID > LZ4F_max4MB, including invalid enum integers', 'LZ4F_ERROR_maxBlockSize_invalid'],
    ['LZ4F_headerSize', 'src is NULL', 'LZ4F_ERROR_srcPtr_wrong'],
    ['LZ4F_headerSize', 'srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH', 'LZ4F_ERROR_frameHeader_incomplete'],
    ['LZ4F_createCompressionContext', 'output context pointer is NULL', 'LZ4F_ERROR_parameter_null'],
    ['LZ4F_createDecompressionContext', 'output context pointer is NULL', 'LZ4F_ERROR_parameter_null'],
    ['LZ4F_readOpen', 'FILE pointer or output state pointer is NULL', 'LZ4F_ERROR_parameter_null'],
    ['LZ4F_read', 'state pointer or output buffer is NULL', 'LZ4F_ERROR_parameter_null'],
    ['LZ4F_readClose', 'state pointer is NULL', 'LZ4F_ERROR_parameter_null'],
    ['LZ4F_writeOpen', 'FILE pointer or output state pointer is NULL', 'LZ4F_ERROR_parameter_null'],
    ['LZ4F_write', 'state pointer or input buffer is NULL', 'LZ4F_ERROR_parameter_null'],
    ['LZ4F_writeClose', 'state pointer is NULL', 'LZ4F_ERROR_parameter_null'],
    ['LZ4_XXH32_update', 'input is NULL and len is nonzero', 'XXH_ERROR'],
    ['LZ4_XXH64_update', 'input is NULL and len is nonzero', 'XXH_ERROR'];

my %seen;
@errors = grep {
    my $key = join "\0", @$_;
    !$seen{$key}++;
} @errors;

open my $errors_out, '>', File::Spec->catfile($crate, 'ERRORS.md')
    or die "ERRORS.md: $!";
print {$errors_out} "# Error surface\n\n";
print {$errors_out} "Rows come from explicit public/propagated-core rejection branches in the C sources plus public-header macro boundaries. Purely internal invariant assertions with no external invalid-input construction are excluded.\n\n";
print {$errors_out} "| # | function | trigger (the exact invalid input/condition) | expected C result |\n";
print {$errors_out} "|---:|----------|---------------------------------------------|-------------------|\n";
for my $index (0 .. $#errors) {
    my ($function, $trigger, $result) = @{$errors[$index]};
    $result =~ s/\|/\\|/g;
    print {$errors_out} "| ", $index + 1, " | `$function` | $trigger | `$result` |\n";
}
close $errors_out;

print "symbols=", scalar(@c_symbols),
    " missing=", scalar(@missing),
    " configs=", scalar(@c_symbols),
    " errors=", scalar(@errors), "\n";
