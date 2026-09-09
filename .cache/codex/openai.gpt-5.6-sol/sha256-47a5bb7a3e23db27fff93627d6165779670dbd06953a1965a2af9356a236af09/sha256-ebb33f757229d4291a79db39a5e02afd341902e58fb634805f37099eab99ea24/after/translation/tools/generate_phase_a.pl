#!/usr/bin/env perl
use strict;
use warnings;
use File::Basename qw(dirname);
use File::Spec;
use Cwd qw(abs_path);

my $crate = abs_path(File::Spec->catdir(dirname(__FILE__), '..'));
my $root = abs_path(File::Spec->catdir($crate, '..'));
my $c_so = File::Spec->catfile($root, 'c_src', 'build', 'liblz4.so');
my $rust_so = File::Spec->catfile($crate, 'target', 'release', 'liblz4.so');
my @sources = map { File::Spec->catfile($root, 'c_src', 'src', $_) }
    qw(lz4.c lz4hc.c lz4frame.c lz4file.c xxhash.c);

sub command_lines {
    my (@command) = @_;
    open my $fh, '-|', @command or die "run @command: $!";
    my @lines = <$fh>;
    close $fh or die "@command failed";
    chomp @lines;
    return @lines;
}

sub symbols {
    my ($path) = @_;
    my @lines = command_lines('nm', '-D', '--defined-only', $path);
    my %symbols;
    for my $line (@lines) {
        my @fields = split /\s+/, $line;
        $symbols{$fields[-1]} = 1 if @fields;
    }
    return %symbols;
}

my %c_symbols = symbols($c_so);
my %rust_symbols = symbols($rust_so);

open my $symbols_md, '>', File::Spec->catfile($crate, 'SYMBOLS.md')
    or die "write SYMBOLS.md: $!";
print {$symbols_md} "# Dynamic symbol surface\n\n";
print {$symbols_md} "Generated mechanically from `nm -D --defined-only` on both shared libraries.\n\n";
print {$symbols_md} "| # | C symbol | Rust export | Status |\n";
print {$symbols_md} "|---:|----------|-------------|:------:|\n";
my $symbol_index = 0;
for my $symbol (sort keys %c_symbols) {
    ++$symbol_index;
    my $present = exists $rust_symbols{$symbol};
    print {$symbols_md} "| $symbol_index | `$symbol` | ",
        ($present ? "`$symbol`" : "MISSING"), " | ",
        ($present ? "[x]" : "[ ]"), " |\n";
}
my @missing = grep { !exists $rust_symbols{$_} } sort keys %c_symbols;
my @extra = grep { !exists $c_symbols{$_} } sort keys %rust_symbols;
print {$symbols_md} "\n## Diff summary\n\n";
print {$symbols_md} "- C exports: ", scalar(keys %c_symbols), "\n";
print {$symbols_md} "- Rust exports: ", scalar(keys %rust_symbols), "\n";
print {$symbols_md} "- Missing from Rust: ", scalar(@missing), "\n";
print {$symbols_md} "- Rust-only exports: ", scalar(@extra), "\n";
close $symbols_md or die "close SYMBOLS.md: $!";

# Build a source-line-to-function map from ctags. This is only used for
# provenance labels; rejection snippets themselves come directly from source.
my %functions_by_file;
for my $line (command_lines('ctags', '-x', '--c-kinds=f', @sources)) {
    if ($line =~ /^(\S+)\s+function\s+(\d+)\s+(\S+)/) {
        push @{$functions_by_file{abs_path($3)}}, [$2 + 0, $1];
    }
}
for my $file (keys %functions_by_file) {
    @{$functions_by_file{$file}} =
        sort { $a->[0] <=> $b->[0] } @{$functions_by_file{$file}};
}

sub enclosing_function {
    my ($file, $line_number) = @_;
    my $function = '(file scope)';
    for my $entry (@{$functions_by_file{$file} // []}) {
        last if $entry->[0] > $line_number;
        $function = $entry->[1];
    }
    return $function;
}

sub clean {
    my ($text) = @_;
    $text =~ s/^\s+|\s+$//g;
    $text =~ s/\|/\\|/g;
    $text =~ s/`/'/g;
    $text =~ s/\s+/ /g;
    return $text;
}

my @errors;
for my $file (@sources) {
    open my $source, '<', $file or die "read $file: $!";
    my @lines = <$source>;
    close $source;
    for (my $i = 0; $i < @lines; ++$i) {
        my $line_number = $i + 1;
        my $line = $lines[$i];
        my $snippet = clean($line);
        next if $snippet =~ /^#/;
        my ($trigger, $result);

        if ($line =~ /RETURN_ERROR_IF\s*\((.*),\s*([A-Za-z0-9_]+)\s*\)/) {
            ($trigger, $result) = (clean($1), "encoded LZ4F_ERROR_$2");
        } elsif ($line =~ /if\s*\((.*?)\).*RETURN_ERROR\s*\(\s*([A-Za-z0-9_]+)\s*\)/) {
            ($trigger, $result) = (clean($1), "encoded LZ4F_ERROR_$2");
        } elsif ($line =~ /RETURN_ERROR\s*\(\s*([A-Za-z0-9_]+)\s*\)/) {
            my $context = '';
            for my $back (reverse 1 .. 3) {
                next if $i - $back < 0;
                my $candidate = clean($lines[$i - $back]);
                if ($candidate =~ /\bif\b|\bcase\b/) {
                    $context = $candidate;
                    last;
                }
            }
            ($trigger, $result) = ($context || "branch at $snippet", "encoded LZ4F_ERROR_$1");
        } elsif ($line =~ /\bassert\s*\((.*)\)\s*;/) {
            ($trigger, $result) = ("assertion `$1` is false", "assertion failure when assertions are enabled");
        } elsif ($line =~ /if\s*\((.*?)\).*return\s+(-1|0|NULL)\s*;/) {
            ($trigger, $result) = (clean($1), $2);
        } elsif ($line =~ /return\s+(-1|0|NULL)\s*;.*(?:invalid|error|fail|overflow|not enough|cannot|unsupported)/i) {
            my $context = '';
            for my $back (reverse 1 .. 4) {
                next if $i - $back < 0;
                my $candidate = clean($lines[$i - $back]);
                if ($candidate =~ /\bif\b/) {
                    $context = $candidate;
                    last;
                }
            }
            ($trigger, $result) = ($context || $snippet, $1);
        } elsif ($line =~ /\bif\s*\(([^)]*(?:NULL|<\s*0|>\s*LZ4[A-Z0-9_]*|>=\s*LZ4[A-Z0-9_]*|<=\s*0)[^)]*)\)/) {
            my $block = join ' ', map { clean($lines[$_]) }
                $i .. (($i + 3 < $#lines) ? $i + 3 : $#lines);
            if ($block =~ /(return\s+(?:-1|0|NULL)|RETURN_ERROR(?:_IF)?\s*\()/) {
                ($trigger, $result) = (clean($1), clean($block));
            }
        }
        next unless defined $trigger;
        push @errors, {
            file => File::Basename::basename($file),
            line => $line_number,
            function => enclosing_function(abs_path($file), $line_number),
            trigger => $trigger,
            result => $result,
        };
    }
}

# Publicly documented scalar boundaries which are represented as macros rather
# than standalone source branches.
push @errors,
    { file => 'lz4.h', line => 214, function => 'LZ4_compressBound',
      trigger => 'inputSize < 0', result => '0' },
    { file => 'lz4.h', line => 214, function => 'LZ4_compressBound',
      trigger => 'inputSize > LZ4_MAX_INPUT_SIZE (0x7E000000)', result => '0' },
    { file => 'lz4.h', line => 302, function => 'LZ4_decompress_safe_partial',
      trigger => 'targetOutputSize > dstCapacity', result => 'negative decode error' },
    { file => 'lz4frame.h', line => 408, function => 'LZ4F_headerSize',
      trigger => 'srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH (5)', result => 'encoded LZ4F_ERROR_frameHeader_incomplete' },
    { file => 'lz4frame.h', line => 266, function => 'LZ4F_createCompressionContext',
      trigger => 'version != LZ4F_VERSION (100)', result => 'implementation accepts version; output otherwise follows C exactly' },
    { file => 'lz4frame.h', line => 386, function => 'LZ4F_createDecompressionContext',
      trigger => 'version != LZ4F_VERSION (100)', result => 'implementation accepts version; output otherwise follows C exactly' };

@errors = sort {
    $a->{file} cmp $b->{file}
        || $a->{line} <=> $b->{line}
        || $a->{function} cmp $b->{function}
} @errors;

open my $errors_md, '>', File::Spec->catfile($crate, 'ERRORS.md')
    or die "write ERRORS.md: $!";
print {$errors_md} "# Error surface\n\n";
print {$errors_md} "Mechanically derived from runtime rejection macros/returns, null and range checks, assertions, and public boundary macros. Source locations are retained to prevent branch collapsing.\n\n";
print {$errors_md} "| # | function | trigger (exact invalid input/condition) | expected C result | source | [ ] |\n";
print {$errors_md} "|---:|----------|-----------------------------------------|-------------------|--------|:---:|\n";
my $error_index = 0;
for my $error (@errors) {
    ++$error_index;
    print {$errors_md} "| $error_index | `$error->{function}` | $error->{trigger} | $error->{result} | `$error->{file}:$error->{line}` | [ ] |\n";
}
close $errors_md or die "close ERRORS.md: $!";

sub configuration_for {
    my ($symbol) = @_;
    return 'no-input metadata/version result'
        if $symbol =~ /(?:versionNumber|versionString|getVersion|compressionLevel_max|sizeofState|sizeofStreamState)$/;
    return 'error-code domain: success, each encoded error, arbitrary non-error size_t'
        if $symbol =~ /LZ4F_(?:isError|getErrorName|getErrorCode)$/;
    return 'block-size enum: default(0), 4, 5, 6, 7'
        if $symbol eq 'LZ4F_getBlockSize';
    return 'one-shot hash: lengths 0, 1, word-1, word, stripe-1, stripe, stripe+1, many; aligned/unaligned input; seeds 0 and nonzero'
        if $symbol =~ /^LZ4_XXH(?:32|64)$/;
    return 'stream hash lifecycle: create/reset; zero/one/many updates with varied segmentation and unaligned chunks; digest midstream/final; copy state'
        if $symbol =~ /^LZ4_XXH(?:32|64)_(?:createState|freeState|reset|update|digest|copyState)$/;
    return 'canonical hash conversion: zero, all-ones, random values; big-endian byte representation and inverse'
        if $symbol =~ /^LZ4_XXH(?:32|64)_(?:canonicalFromHash|hashFromCanonical)$/;
    return 'frame one-shot: NULL/default and explicit preferences; block size 0/4/5/6/7; linked/independent; checksums off/on; content size unknown/known; dictID 0/nonzero; compression level negative/0/HC; autoFlush 0/1; favorDecSpeed 0/1; empty/small/block-boundary/multiblock/random input'
        if $symbol =~ /^LZ4F_compressFrame(?:Bound|_usingCDict)?$/;
    return 'frame streaming compression lifecycle: context create; begin; update zero/one/many chunks; compressed and uncompressed update; flush empty/nonempty; end; stableSrc 0/1; default/full preferences; dictionary none/raw/CDict'
        if $symbol =~ /^LZ4F_(?:createCompressionContext|createCompressionContext_advanced|freeCompressionContext|compressBegin|compressBegin_internal|compressBegin_usingDict|compressBegin_usingDictOnce|compressBegin_usingCDict|compressBound|compressUpdate|uncompressedUpdate|flush|compressEnd)$/;
    return 'frame streaming decompression lifecycle: context create; header split at every byte; body split/random chunks; output capacities 0/small/exact/large; stableDst 0/1; skipChecksums 0/1; dictionary none/present; reset/reuse'
        if $symbol =~ /^LZ4F_(?:createDecompressionContext|createDecompressionContext_advanced|freeDecompressionContext|headerSize|getFrameInfo|decompress|decompress_usingDict|resetDecompressionContext)$/;
    return 'digested dictionary lifecycle: dict lengths 0, short, 64KiB, >64KiB; default/custom allocator; NULL free'
        if $symbol =~ /^LZ4F_(?:createCDict|createCDict_advanced|freeCDict)$/;
    return 'FILE* lifecycle: open/read-or-write/close; empty, small, multiblock files; default and explicit frame preferences'
        if $symbol =~ /^LZ4F_(?:readOpen|read|readClose|writeOpen|write|writeClose)$/;
    return 'block compression: empty/small/MINMATCH boundaries/255/64KiB/random and repetitive input; destination exact bound/tight; acceleration <=0/1/2/max/>max'
        if $symbol =~ /^LZ4_compress(?:_default|_fast|Bound|$|_limitedOutput)$/;
    return 'external-state block compression: aligned allocated state; fresh and fast-reset reuse; input/capacity/acceleration matrix'
        if $symbol =~ /^LZ4_compress_(?:fast_extState|fast_extState_fastReset|withState|limitedOutput_withState)$/;
    return 'fill-output compression: source sizes empty/small/large; destination sizes 1/small/exact/bound; verify consumed source count and round trip'
        if $symbol =~ /^LZ4_compress_(?:destSize|destSize_extState)$/;
    return 'safe block decompression: valid blocks from all compressor modes; empty/small/exact/oversized destination; malformed/truncated/random compressed input'
        if $symbol =~ /^LZ4_decompress_safe$/;
    return 'partial safe decompression: targets 0/1/middle/exact/>decoded, capacities target/exact/large; dictionary none/prefix/external'
        if $symbol =~ /^LZ4_decompress_safe_partial/;
    return 'dictionary safe decompression: no dictionary; <64KiB, 64KiB and >64KiB dictionaries; prefix-adjacent and external dictionary layouts'
        if $symbol =~ /^LZ4_decompress_safe_(?:usingDict|forceExtDict|withPrefix64k)$/;
    return 'legacy fast decompression: exact original size; no/prefix/external dictionary; streaming continuation'
        if $symbol =~ /^LZ4_decompress_fast/ || $symbol =~ /^LZ4_uncompress/;
    return 'fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix'
        if $symbol =~ /^LZ4_(?:createStream|freeStream|initStream|resetStream|resetStream_fast|loadDict|loadDictSlow|loadDict_internal|attach_dictionary|compress_fast_continue|compress_continue|compress_limitedOutput_continue|compress_forceExtDict|saveDict|create|resetStreamState|slideInputBuffer)$/;
    return 'stream decoding lifecycle: create/set dictionary/reset; one/many blocks; contiguous/noncontiguous/ring output; dictionary none/short/64KiB'
        if $symbol =~ /^LZ4_(?:createStreamDecode|freeStreamDecode|setStreamDecode|decoderRingBufferSize|decompress_safe_continue)$/;
    return 'HC block compression: levels below-min/2/9/10/12/>12; favor-decode off/on where applicable; empty/small/large random and repetitive input; bound/tight destination'
        if $symbol =~ /^LZ4_compress(?:_HC|HC)/ && $symbol !~ /continue|destSize|extState/;
    return 'HC external state/fill-output: aligned state; fresh/fast-reset reuse; levels 2/9/10/12; destination exact/tight; consumed source count'
        if $symbol =~ /^LZ4_compress_HC_(?:extStateHC|extStateHC_fastReset|destSize)$/;
    return 'HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output'
        if $symbol =~ /^LZ4_(?:createStreamHC|freeStreamHC|initStreamHC|resetStreamHC|resetStreamHC_fast|setCompressionLevel|favorDecompressionSpeed|loadDictHC|attach_HC_dictionary|compress_HC_continue|compress_HC_continue_destSize|compressHC_continue|compressHC_limitedOutput_continue|compressHC2_continue|compressHC2_limitedOutput_continue|saveDictHC|createHC|freeHC|resetStreamStateHC|slideInputBufferHC)$/;
    return 'internal HC dictionary search: no dictionary and external dictionary; MINMATCH and distance boundaries'
        if $symbol eq 'LZ4HC_searchExtDict';
    return 'public lifecycle/compatibility wrapper exercised with valid zero, boundary, and randomized inputs appropriate to its C prototype';
}

open my $configs_md, '>', File::Spec->catfile($crate, 'CONFIGS.md')
    or die "write CONFIGS.md: $!";
print {$configs_md} "# Configuration surface\n\n";
print {$configs_md} "One mechanically enumerated row per exported C entry point. Each row records the option and input-shape axes selected from the public headers and the implementation branches for that family. Shared lifecycle rows are intentionally repeated per entry point so no low-level or compatibility export disappears from coverage.\n\n";
print {$configs_md} "| # | entry point(s) | configuration (options set + input shape) | [ ] |\n";
print {$configs_md} "|---:|----------------|--------------------------------------------|:---:|\n";
my $config_index = 0;
for my $symbol (sort keys %c_symbols) {
    ++$config_index;
    my $configuration = configuration_for($symbol);
    print {$configs_md} "| $config_index | `$symbol` | $configuration | [ ] |\n";
}
print {$configs_md} "\n## Build-time configuration\n\n";
print {$configs_md} "Cargo declares no features. The only Rust feature sets are therefore the equivalent default and `--no-default-features` builds. CMake and `build.rs` both fix `XXH_NAMESPACE=LZ4_`, `LZ4_HEAPMODE=0`, and `LZ4F_HEAPMODE=0`.\n";
close $configs_md or die "close CONFIGS.md: $!";
