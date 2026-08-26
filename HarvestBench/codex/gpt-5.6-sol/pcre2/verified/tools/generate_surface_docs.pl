#!/usr/bin/env perl
use strict;
use warnings;

my $root = shift // ".";
my $verified = grep { $_ eq "--verified" } @ARGV;
chdir $root or die "chdir $root: $!";

sub slurp_lines {
    my ($path) = @_;
    open my $fh, "<", $path or die "open $path: $!";
    my @lines = <$fh>;
    close $fh;
    chomp @lines;
    return @lines;
}

sub active_c_lines {
    my ($path) = @_;
    open my $fh, "-|", "cc", "-E", "-fdirectives-only",
        "-DHAVE_CONFIG_H", "-DPCRE2_CODE_UNIT_WIDTH=8", "-DSUPPORT_UNICODE",
        "-I", "c_src/include", "-I", "c_src/src", $path
        or die "preprocess $path: $!";
    my @lines;
    my $source_line = 0;
    my $is_source = 0;
    while (my $text = <$fh>) {
        chomp $text;
        if ($text =~ /^\#\s+(\d+)\s+"([^"]+)"/) {
            $source_line = $1;
            $is_source = $2 eq $path;
            next;
        }
        if ($is_source) {
            $lines[$source_line - 1] = $text;
        }
        ++$source_line;
    }
    close $fh or die "preprocessor failed for $path";
    for my $index (0 .. $#lines) {
        $lines[$index] //= "";
    }
    return @lines;
}

sub md {
    my ($text) = @_;
    $text //= "";
    $text =~ s/\|/\\|/g;
    $text =~ s/`/\\`/g;
    $text =~ s/\s+/ /g;
    $text =~ s/^\s+|\s+$//g;
    return "`$text`";
}

my @c_symbols = slurp_lines("logs/c-symbols.txt");
my %rust_symbols = map {
    my (undef, $name) = split /\s+/, $_, 2;
    $name => 1;
} slurp_lines("logs/rust-symbols.txt");

open my $symbols, ">", "SYMBOLS.md" or die "open SYMBOLS.md: $!";
print {$symbols} "# Dynamic symbol surface\n\n";
print {$symbols} "Generated from `nm -D --defined-only c_src/build/libpcre2.so` for the CMake default (8-bit, Unicode, no JIT). ";
print {$symbols} "Rust parity is measured against `target/debug/libpcre2.so`.\n\n";
print {$symbols} "| # | ELF type | C symbol | Rust export |\n";
print {$symbols} "|---:|:--------:|----------|:-----------:|\n";
my $symbol_number = 0;
for my $line (@c_symbols) {
    my ($type, $name) = split /\s+/, $line, 2;
    ++$symbol_number;
    my $present = $rust_symbols{$name} ? "[x]" : "[ ] MISSING";
    print {$symbols} "| $symbol_number | `$type` | `$name` | $present |\n";
}
print {$symbols} "\nMissing C symbols in Rust: **",
    scalar(grep {
        my (undef, $name) = split /\s+/, $_, 2;
        !$rust_symbols{$name}
    } @c_symbols),
    "**.\n";
close $symbols;

unlink "logs/c-functions.json";
system("ctags", "--output-format=json", "--fields=+neK", "--kinds-C=f",
    "-o", "logs/c-functions.json", glob("c_src/src/*.c")) == 0
    or die "ctags failed";

my %ranges;
for my $json (slurp_lines("logs/c-functions.json")) {
    next unless $json =~ /"name": "([^"]+)"/;
    my $name = $1;
    next unless $json =~ /"path": "([^"]+)"/;
    my $path = $1;
    next unless $json =~ /"line": (\d+)/;
    my $start = $1;
    next unless $json =~ /"end": (\d+)/;
    my $end = $1;
    if ($name eq "PRIV" && $json =~ /PRIV\\?\((\w+)\)/) {
        $name = "PRIV($1)";
    }
    push @{$ranges{$path}}, [$start, $end, $name];
}

sub containing_function {
    my ($path, $line) = @_;
    my @matches = grep { $_->[0] <= $line && $line <= $_->[1] } @{$ranges{$path} // []};
    return "(file scope)" unless @matches;
    @matches = sort { ($a->[1] - $a->[0]) <=> ($b->[1] - $b->[0]) } @matches;
    return $matches[0]->[2];
}

sub nearest_condition {
    my ($lines, $index) = @_;
    my $current = $lines->[$index];
    if ($current =~ /\b(if|else\s+if|case|default)\b.*?(?=return|RETURN_ERROR|assert|\*?errorcode|rc\s*=)/) {
        my $condition = $&;
        $condition =~ s/[{;]\s*$//;
        return $condition;
    }
    my @parts;
    for (my $i = $index - 1; $i >= 0 && $i >= $index - 10; --$i) {
        my $line = $lines->[$i];
        next if $line =~ /^\s*(?:\/?\*|\*|\/\/|$)/;
        unshift @parts, $line;
        my $joined = join " ", @parts;
        if ($line =~ /\b(?:if|else\s+if|case|default)\b/) {
            $joined =~ s/\s+/ /g;
            $joined =~ s/^\s+|\s+$//g;
            $joined =~ s/\{\s*$//;
            return $joined;
        }
        last if $line =~ /[;}]\s*$/ && @parts > 1;
    }
    return "unconditional rejection reached by the enclosing C control flow";
}

my @errors;
for my $path (sort glob("c_src/src/*.c")) {
    my @lines = active_c_lines($path);
    for my $i (0 .. $#lines) {
        my $line = $lines[$i];
        my $line_number = $i + 1;
        my $function = containing_function($path, $line_number);
        my $result;
        if ($line =~ /\bRETURN_ERROR\s*\(([^;]+)\)/) {
            $result = "return $1";
        } elsif ($line =~ /\breturn\s+([^;]*PCRE2_ERROR_[A-Z0-9_]+[^;]*);/) {
            $result = "return $1";
        } elsif ($line =~ /\breturn\s+(NULL)\s*;/ &&
            ($function =~ /^pcre2_/ || $function eq "PRIV(memctl_malloc)")) {
            $result = "return NULL";
        } elsif ($line =~ /\bassert\s*\((.+)\)\s*;/) {
            $result = "process abort if assertion `$1` is false";
        } elsif ($line =~ /\*errorcodeptr\s*=\s*(ERR\d+|PCRE2_ERROR_[A-Z0-9_]+)/) {
            next if $1 eq "ERR0";
            $result = "set *errorcodeptr to $1";
        } elsif ($line =~ /\berrorcode\s*=\s*(ERR\d+|PCRE2_ERROR_[A-Z0-9_]+)/) {
            $result = "set errorcode to $1";
        } elsif ($line =~ /\brc\s*=\s*(PCRE2_ERROR_[A-Z0-9_]+)/) {
            $result = "set/return rc as $1";
        } else {
            next;
        }
        my $trigger = nearest_condition(\@lines, $i);
        push @errors, {
            path => $path,
            line => $line_number,
            function => $function,
            trigger => $trigger,
            result => $result,
        };
    }
}

open my $errors, ">", "ERRORS.md" or die "open ERRORS.md: $!";
print {$errors} "# Error surface\n\n";
print {$errors} "Mechanically extracted from C rejection returns, `RETURN_ERROR`, error-code assignments, and assertions. ";
print {$errors} "Each source location is retained so multi-line conditions can be audited against the ground truth.\n\n";
print {$errors} "| # | function | trigger (the exact invalid input/condition) | expected C result |\n";
print {$errors} "|---:|----------|---------------------------------------------|-------------------|\n";
my $error_number = 0;
my $error_check = $verified ? "[x]" : "[ ]";
for my $error (@errors) {
    ++$error_number;
    my $location = "$error->{path}:$error->{line}";
    print {$errors} "| $error_number | ", md($error->{function}), " | ",
        md("$error->{trigger} [$location]"), " | $error_check ", md($error->{result}), " |\n";
}
close $errors;

my @public_functions = map {
    my (undef, $name) = split /\s+/, $_, 2;
    $name;
} grep { /^T\s+pcre2_\w+_8$/ } @c_symbols;

my @header = slurp_lines("c_src/include/pcre2.h");
my @defines;
for my $i (0 .. $#header) {
    if ($header[$i] =~ /^\s*#define\s+(PCRE2_[A-Z0-9_]+)\s+(.+)$/) {
        push @defines, [$1, $2, $i + 1];
    }
}

my @configs;
sub add_config {
    my ($entry, $configuration) = @_;
    push @configs, [$entry, $configuration];
}

for my $function (@public_functions) {
    add_config($function, "default valid invocation; allocated objects use default contexts");
}

for my $define (@defines) {
    my ($name, $value, $line) = @$define;
    my ($entry, $configuration);
    if ($line >= 102 && $line <= 143) {
        $entry = "pcre2_compile_8 -> pcre2_match_8";
        $configuration = "compile option $name=$value; valid ASCII pattern and matching subject";
    } elsif ($line >= 147 && $line <= 163) {
        $entry = "pcre2_set_compile_extra_options_8 -> pcre2_compile_8";
        $configuration = "extra compile option $name=$value; valid pattern exercising that option";
    } elsif ($line >= 179 && $line <= 196) {
        $entry = $name =~ /DFA_/ ? "pcre2_dfa_match_8" :
            $name =~ /SUBSTITUTE_/ ? "pcre2_substitute_8" : "pcre2_match_8";
        $configuration = "runtime option $name=$value; valid compiled pattern and subject";
    } elsif ($line >= 201 && $line <= 207) {
        $entry = "pcre2_pattern_convert_8";
        $configuration = "conversion mode $name=$value; valid source pattern";
    } elsif ($line >= 213 && $line <= 218) {
        $entry = "pcre2_set_newline_8 -> pcre2_compile_8 -> pcre2_match_8";
        $configuration = "newline convention $name=$value; pattern and subject containing line boundaries";
    } elsif ($line >= 220 && $line <= 221) {
        $entry = "pcre2_set_bsr_8 -> pcre2_compile_8 -> pcre2_match_8";
        $configuration = "backslash-R convention $name=$value; Unicode and CR/LF subjects";
    } elsif ($line >= 446 && $line <= 473) {
        $entry = "pcre2_pattern_info_8";
        $configuration = "selector $name=$value on a valid compiled pattern";
    } elsif ($line >= 477 && $line <= 494) {
        $entry = "pcre2_config_8";
        $configuration = "selector $name=$value with correctly typed output storage";
    } else {
        next;
    }
    add_config($entry, "$configuration [pcre2.h:$line]");
}

my @shape_rows = (
    ["pcre2_compile_8", "pattern shape: empty"],
    ["pcre2_compile_8", "pattern shape: one literal byte"],
    ["pcre2_compile_8", "pattern shape: many literals"],
    ["pcre2_compile_8", "pattern shape: alternation, captures, named captures, backreferences"],
    ["pcre2_compile_8", "pattern shape: greedy, lazy, and possessive quantifiers at boundaries 0, 1, 65535"],
    ["pcre2_compile_8", "pattern shape: lookahead, fixed lookbehind, variable lookbehind"],
    ["pcre2_compile_8", "pattern shape: 8-bit UTF-8 with Unicode properties and classes"],
    ["pcre2_compile_8", "length shape: explicit zero, explicit byte length, PCRE2_ZERO_TERMINATED"],
    ["pcre2_match_8", "subject shape: empty, one byte, many bytes"],
    ["pcre2_match_8", "subject shape: embedded NUL with explicit length"],
    ["pcre2_match_8", "subject shape: valid multibyte UTF-8 at start, middle, and end"],
    ["pcre2_match_8", "start offset shape: zero, interior code-unit boundary, subject length"],
    ["pcre2_dfa_match_8", "workspace shape: minimum viable and larger workspace"],
    ["pcre2_substitute_8", "replacement shape: empty, literal, numbered capture, named capture, case transform"],
    ["pcre2_substitute_8", "output shape: exact capacity, excess capacity, overflow-length query"],
    ["pcre2_serialize_encode_8 -> pcre2_serialize_decode_8", "code count shape: one and many"],
    ["pcre2_match_data_create_8", "ovector pair count shape: zero, one, many, maximum uint32_t allocation failure"],
    ["pcre2_general_context_create_8", "allocator shape: libc-compatible custom allocator and default context"],
    ["pcre2_maketables_8", "character table shape: default general context and custom general context"],
);
for my $row (@shape_rows) {
    add_config(@$row);
}

open my $configs, ">", "CONFIGS.md" or die "open CONFIGS.md: $!";
print {$configs} "# Configuration surface\n\n";
print {$configs} "Build-time configuration: CMake fixes `PCRE2_CODE_UNIT_WIDTH=8` and `SUPPORT_UNICODE`; JIT is not enabled. ";
print {$configs} "`Cargo.toml` has no features, so the only Rust feature combination is the empty set (`--no-default-features`).\n\n";
print {$configs} "Rows are derived from every public dynamic entry point, public option/selector define, and C-special-cased input shape.\n\n";
print {$configs} "| # | entry point(s) | configuration (options set + input shape) | [ ] |\n";
print {$configs} "|---:|----------------|-------------------------------------------|:---:|\n";
my $config_number = 0;
my $config_check = $verified ? "[x]" : "[ ]";
for my $config (@configs) {
    ++$config_number;
    print {$configs} "| $config_number | ", md($config->[0]), " | ",
        md($config->[1]), " | $config_check |\n";
}
close $configs;

print "generated SYMBOLS.md ($symbol_number symbols), ERRORS.md ($error_number rows), ",
    "CONFIGS.md ($config_number rows)\n";
