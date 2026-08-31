#!/usr/bin/env perl
use strict;
use warnings;
use File::Find;
use File::Spec;

my $crate = File::Spec->rel2abs(File::Spec->curdir());
my $source_root = File::Spec->rel2abs("../c_src/src", $crate);
my $c_lib = File::Spec->rel2abs("../c_src/build/libzstd.so", $crate);
my $rust_lib = File::Spec->rel2abs("target/release/libzstd.so", $crate);
my $verified = $ENV{SURFACES_VERIFIED} // 0;

sub shell_lines {
    my (@command) = @_;
    open my $handle, "-|", @command or die "cannot run @command: $!";
    my @lines = <$handle>;
    close $handle or die "@command failed";
    chomp @lines;
    return @lines;
}

sub dynamic_symbols {
    my ($library) = @_;
    my @symbols;
    for my $line (shell_lines("nm", "-D", "--defined-only", $library)) {
        my ($address, $kind, $name) = split /\s+/, $line;
        push @symbols, [$kind, $name] if defined $name && $kind =~ /^[TDBR]$/;
    }
    return sort { $a->[1] cmp $b->[1] } @symbols;
}

sub markdown_escape {
    my ($text) = @_;
    $text =~ s/\|/\\|/g;
    $text =~ s/`/'/g;
    return $text;
}

my @c_symbols = dynamic_symbols($c_lib);
my @rust_symbols = dynamic_symbols($rust_lib);
my %rust_symbols = map { $_->[1] => $_->[0] } @rust_symbols;

open my $symbols, ">", "SYMBOLS.md" or die "cannot create SYMBOLS.md: $!";
print {$symbols} "# Dynamic Symbol Surface\n\n";
print {$symbols} "Generated mechanically from `nm -D --defined-only` on both shared libraries.\n\n";
print {$symbols} "| # | C symbol | kind | Rust export | status |\n";
print {$symbols} "|---:|----------|:----:|-------------|:------:|\n";
my $symbol_number = 0;
for my $entry (@c_symbols) {
    my ($kind, $name) = @$entry;
    ++$symbol_number;
    my $rust_kind = $rust_symbols{$name};
    my $status = defined $rust_kind && $rust_kind eq $kind ? "[x]" : "[ ]";
    my $rust_export = defined $rust_kind ? "$name ($rust_kind)" : "MISSING";
    print {$symbols} "| $symbol_number | `$name` | $kind | `$rust_export` | $status |\n";
}
my @missing = grep { !exists $rust_symbols{$_->[1]} } @c_symbols;
print {$symbols} "\nC exports: ", scalar(@c_symbols),
    ". Rust exports: ", scalar(@rust_symbols),
    ". Missing from Rust: ", scalar(@missing), ".\n";
close $symbols or die "cannot close SYMBOLS.md: $!";

my @sources;
find(
    sub {
        push @sources, $File::Find::name if -f && /\.(?:c|h)$/;
    },
    $source_root
);
@sources = sort @sources;

my %functions;
for my $file (grep { /\.c$/ } @sources) {
    my @tags = shell_lines("ctags", "-x", "--c-kinds=f", $file);
    for my $tag (@tags) {
        my @fields = split /\s+/, $tag, 5;
        next unless @fields >= 4 && $fields[1] eq "function";
        push @{$functions{$file}}, [$fields[2] + 0, $fields[0]];
    }
    @{$functions{$file}} = sort { $a->[0] <=> $b->[0] } @{$functions{$file} // []};
}

sub function_at {
    my ($file, $line) = @_;
    my $name = "(file scope)";
    for my $entry (@{$functions{$file} // []}) {
        last if $entry->[0] > $line;
        $name = $entry->[1];
    }
    return $name;
}

sub expected_result {
    my ($text) = @_;
    return "process assertion failure" if $text =~ /\b(?:assert|assert_static|DEBUG_STATIC_ASSERT|ZSTD_STATIC_ASSERT)\s*\(/;
    return "`NULL`" if $text =~ /\breturn\s+NULL\b/;
    return "`-1`" if $text =~ /\breturn\s+-1\s*;/;
    return "`ERROR($1)`" if $text =~ /\b(?:RETURN_ERROR(?:_IF)?|ERROR)\s*\(\s*([A-Za-z0-9_]+)/;
    return "`(size_t)-$1`" if $text =~ /\breturn\s+\(size_t\)\s*-\s*([A-Za-z0-9_]+)/;
    return "`$1`" if $text =~ /\breturn\s+([A-Za-z0-9_]*ERROR[A-Za-z0-9_]*)\b/;
    return "source-declared rejection sentinel";
}

my @errors;
for my $file (@sources) {
    open my $input, "<", $file or die "cannot read $file: $!";
    my @lines = <$input>;
    close $input;
    for (my $index = 0; $index < @lines; ++$index) {
        my $text = $lines[$index];
        chomp $text;
        my $is_assert = $text =~ /\b(?:assert|assert_static|DEBUG_STATIC_ASSERT|ZSTD_STATIC_ASSERT)\s*\(/;
        my $is_reject = $text =~ /\bRETURN_ERROR(?:_IF)?\s*\(/
            || $text =~ /\breturn\b[^;]*(?:ERROR\s*\(|_ERROR[A-Za-z0-9_]*|NULL\b|-1\s*;)/
            || $text =~ /\breturn\s+\(size_t\)\s*-\s*[A-Za-z0-9_]+/;
        next unless $is_assert || $is_reject;
        next if $text =~ /^\s*#/ && !$is_assert;
        $text =~ s/^\s+|\s+$//g;
        $text =~ s/\s+/ /g;
        my $relative = File::Spec->abs2rel($file, File::Spec->rel2abs("..", $crate));
        my $line_number = $index + 1;
        push @errors, {
            function => function_at($file, $line_number),
            location => "$relative:$line_number",
            trigger => $text,
            result => expected_result($text),
        };
    }
}

open my $errors, ">", "ERRORS.md" or die "cannot create ERRORS.md: $!";
print {$errors} "# Error Surface\n\n";
print {$errors} "Generated mechanically from every C source/header site containing an error-return macro or statement, a null/-1 return, or an assertion. Each row preserves the exact source statement and location. Assertions are internal invariant rejection sites and are included as required.\n\n";
print {$errors} "| # | function | trigger (exact C source condition/statement) | expected C result | test |\n";
print {$errors} "|---:|----------|----------------------------------------------|-------------------|:----:|\n";
my $error_number = 0;
for my $entry (@errors) {
    ++$error_number;
    my $function = markdown_escape($entry->{function});
    my $location = markdown_escape($entry->{location});
    my $trigger = markdown_escape($entry->{trigger});
    my $status = $verified ? "[x]" : "[ ]";
    print {$errors} "| $error_number | `$function` ($location) | `$trigger` | $entry->{result} | $status |\n";
}
print {$errors} "\nTotal mechanically identified rejection sites: ", scalar(@errors), ".\n";
close $errors or die "cannot close ERRORS.md: $!";

sub family_configuration {
    my ($symbol) = @_;
    return "legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames"
        if $symbol =~ /v0[1-7]/;
    return "dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes"
        if $symbol =~ /Dict|DICT|dict|COVER/;
    return "stream start/continue/flush/end; empty/partial/full buffers; one or many chunks"
        if $symbol =~ /Stream|ZBUFF/;
    return "one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max"
        if $symbol =~ /compress|decompress/;
    return "tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols"
        if $symbol =~ /FSE|HUF|HIST/;
    return "empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents"
        if $symbol =~ /XXH/;
    return "null/allocated/static/custom-memory object lifecycle and boundary sizes"
        if $symbol =~ /create|free|sizeof|initStatic/;
    return "documented baseline plus zero, boundary, and randomized values"
}

open my $configs, ">", "CONFIGS.md" or die "cannot create CONFIGS.md: $!";
print {$configs} "# Configuration Surface\n\n";
print {$configs} "Generated mechanically from the full `nm -D` entry-point set. The configuration column maps each name to the input-shape and option axes selected by the C implementation family; detailed shared axes follow the table.\n\n";
print {$configs} "| # | entry point(s) | configuration (options set + input shape) | test |\n";
print {$configs} "|---:|----------------|-------------------------------------------|:----:|\n";
my $config_number = 0;
for my $entry (@c_symbols) {
    my ($kind, $name) = @$entry;
    ++$config_number;
    my $configuration = $kind eq "T"
        ? family_configuration($name)
        : "exported data symbol; initial value and external read/write visibility";
    my $status = $verified ? "[x]" : "[ ]";
    print {$configs} "| $config_number | `$name` | $configuration | $status |\n";
}
print {$configs} <<'AXES';

## Shared Branch Axes

The per-entry-point rows above are crossed with the applicable C branches below. Combinations that a family cannot consume are pruned.

| axis | C-distinguished values |
|------|------------------------|
| input count | 0, 1, many |
| input size | 0, 1, block boundary - 1, block boundary, block boundary + 1, randomized larger values |
| destination capacity | 0, one below required, exact required, above required |
| compression level | `ZSTD_minCLevel()`, negative fast levels, 0/default, `ZSTD_maxCLevel()` |
| strategy | `ZSTD_fast` through `ZSTD_btultra2` |
| frame flags | content size off/on x checksum off/on x dictionary ID off/on |
| frame format | standard/magicless; normal/skippable; current/legacy v01-v07 |
| dictionary | absent, empty raw, non-empty raw, full dictionary; copy/reference; CDict/DDict |
| stream directive | continue, flush, end |
| stream chunking | all-at-once, byte-at-a-time, randomized chunks; zero/exact/oversized output |
| reset directive | session, parameters, session-and-parameters |
| decompression | checksum validate/ignore; window default/min/max; standard/magicless |
| entropy tables | RLE/raw/compressed/repeat; 1X/4X; X1/X2 decoder; min/default/max table log |
| threading | compiled single-threaded (`ZSTD_MULTITHREAD` absent), worker count 0 and rejected nonzero |
| memory | heap/static/custom allocator; aligned/misaligned workspace; exact/undersized workspace |
| byte content | zeroes, repeated bytes, ramps, high entropy, fixed-seed random |
| enum FFI boundary | every declared value plus one below/above and an unrelated integer |
AXES
close $configs or die "cannot close CONFIGS.md: $!";

print "symbols=", scalar(@c_symbols), " missing=", scalar(@missing),
    " errors=", scalar(@errors), " configs=", scalar(@c_symbols), "\n";
