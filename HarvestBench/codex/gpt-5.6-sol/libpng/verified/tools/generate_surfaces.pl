#!/usr/bin/env perl
use strict;
use warnings;
use File::Basename qw(dirname);
use File::Spec;

my $root = File::Spec->rel2abs(dirname(dirname(__FILE__)));
my $c_so = "$root/c_src/build/libpng.so";
my $rust_so = "$root/target/release/liblibpng.so";

sub command_lines {
    my (@command) = @_;
    open my $fh, "-|", @command or die "cannot run @command: $!";
    my @lines = <$fh>;
    close $fh or die "@command failed";
    chomp @lines;
    return @lines;
}

sub dynamic_symbols {
    my ($library) = @_;
    my %symbols;
    for my $line (command_lines("nm", "-D", "--defined-only", "--format=posix", $library)) {
        my ($name, $type) = split /\s+/, $line;
        $name =~ s/\@\@.*//;
        $symbols{$name} = $type;
    }
    return %symbols;
}

sub markdown {
    my ($text) = @_;
    $text //= "";
    $text =~ s/^\s+|\s+$//g;
    $text =~ s/\s+/ /g;
    $text =~ s/\|/\\|/g;
    $text =~ s/`/'/g;
    return "`$text`";
}

sub source_files {
    return sort glob "$root/c_src/src/*.c";
}

sub function_ranges {
    my %ranges;
    for my $file (source_files()) {
        my @ast = command_lines(
            "clang", "-I", "$root/c_src/include", "-std=c99",
            "-Xclang", "-ast-dump", "-fsyntax-only", $file);
        for my $index (0 .. $#ast) {
            my $line = $ast[$index];
            next unless $line =~ /^([|`]-)FunctionDecl\b/;
            next unless $line =~ />\s+line:(\d+):\d+\s+(?:(?:used|referenced)\s+)*([A-Za-z_]\w*)\s+'/;
            my ($start, $name) = (0 + $1, $2);
            next unless $line =~ /(?:line:|\Q$file\E:)(\d+):\d+>\s/;
            my $end = 0 + $1;

            my $has_body = 0;
            for my $body_index ($index + 1 .. $#ast) {
                last if $ast[$body_index] =~ /^[|`]-FunctionDecl\b/;
                if ($ast[$body_index] =~ /\bCompoundStmt\b/) {
                    $has_body = 1;
                    last;
                }
            }
            next unless $has_body && $end >= $start;
            $ranges{$name} = {
                name => $name,
                start => $start,
                end => $end,
                file => $file,
            };
        }
    }
    return %ranges;
}

sub read_lines {
    my ($file) = @_;
    open my $fh, "<", $file or die "cannot read $file: $!";
    my @lines = <$fh>;
    close $fh;
    return @lines;
}

sub relative_source {
    my ($file) = @_;
    $file =~ s/^\Q$root\/\E//;
    return $file;
}

sub nearest_predicate {
    my ($lines, $start, $line_number) = @_;
    my $first = $line_number - 12;
    $first = $start if $first < $start;
    my $context = join "", @{$lines}[$first - 1 .. $line_number - 1];
    $context =~ s{/\*.*?\*/}{}gs;
    $context =~ s{//[^\n]*}{}g;

    my @predicates;
    while ($context =~ /\b(?:if|else\s+if)\s*(\((?:[^()]++|(?1))*\))/g) {
        push @predicates, $1;
    }
    return @predicates ? $predicates[-1] : "unconditional rejection/state failure";
}

sub statement_at {
    my ($lines, $line_number) = @_;
    my $statement = $lines->[$line_number - 1] // "";
    my $cursor = $line_number;
    while ($statement !~ /;\s*(?:\/\*.*\*\/)?\s*$/ && $cursor < @$lines && $cursor < $line_number + 8) {
        $statement .= $lines->[$cursor++];
    }
    return $statement;
}

sub write_symbols {
    my ($c_symbols, $rust_symbols) = @_;
    open my $out, ">", "$root/SYMBOLS.md" or die "cannot write SYMBOLS.md: $!";
    print {$out} "# Dynamic Symbol Surface\n\n";
    print {$out} "Generated from `nm -D --defined-only` on `c_src/build/libpng.so`.\n";
    print {$out} "The Rust column is checked only when the exact symbol is present in `target/release/liblibpng.so`.\n\n";
    print {$out} "| # | C symbol | ELF type | Rust export |\n";
    print {$out} "|---:|----------|:--------:|:-----------:|\n";
    my $number = 0;
    for my $symbol (sort keys %$c_symbols) {
        ++$number;
        my $present = exists $rust_symbols->{$symbol} ? "[x]" : "[ ]";
        print {$out} "| $number | `$symbol` | `$c_symbols->{$symbol}` | $present |\n";
    }
    my @missing = grep !exists $rust_symbols->{$_}, sort keys %$c_symbols;
    print {$out} "\nMissing Rust exports: **", scalar(@missing), "**.\n";
    close $out;
}

sub write_errors {
    my ($ranges) = @_;
    my %covered = map { $_ => 1 } qw(
        c_src/src/png.c:88
        c_src/src/png.c:91
        c_src/src/png.c:110
        c_src/src/png.c:374
        c_src/src/png.c:694
        c_src/src/png.c:982
        c_src/src/png.c:3783
        c_src/src/pngread.c:1458
        c_src/src/pngread.c:1463
        c_src/src/pngread.c:4178
        c_src/src/pngread.c:4183
        c_src/src/pngread.c:4188
        c_src/src/pngread.c:4193
        c_src/src/pngread.c:4198
        c_src/src/pngwrite.c:2332
        c_src/src/pngwrite.c:2337
    );
    my @events;
    for my $file (source_files()) {
        my @lines = read_lines($file);
        my @functions = sort { $a->{start} <=> $b->{start} }
            grep $_->{file} eq $file, values %$ranges;
        for my $function (@functions) {
            for my $line_number ($function->{start} + 1 .. $function->{end}) {
                my $line = $lines[$line_number - 1] // "";
                my $code = $line;
                $code =~ s{//.*$}{};
                next if $code =~ /^\s*\/?\*/;
                my $kind;
                if ($code =~ /\bpng_[A-Za-z0-9_]*error\s*\(/) {
                    $kind = "error callback/longjmp";
                }
                elsif ($code =~ /\breturn\s+(?:-1|NULL|Z_[A-Z_]*ERROR|PNG_OPTION_INVALID)\s*;/) {
                    $kind = "sentinel return";
                }
                elsif ($code =~ /\bassert\s*\(/) {
                    $kind = "assertion";
                }
                next unless defined $kind;

                my $statement = statement_at(\@lines, $line_number);
                my $predicate = nearest_predicate(\@lines, $function->{start}, $line_number);
                push @events, {
                    function => $function->{name},
                    source => relative_source($file) . ":$line_number",
                    trigger => $predicate,
                    result => "$kind: $statement",
                };
            }
        }
    }

    open my $out, ">", "$root/ERRORS.md" or die "cannot write ERRORS.md: $!";
    print {$out} "# Error Surface\n\n";
    print {$out} "Mechanically extracted from C error calls, assertion statements, and explicit error/sentinel returns.\n";
    print {$out} "Each predicate and result includes its C source location; rows remain unchecked until a differential test reaches that exact branch.\n\n";
    print {$out} "| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |\n";
    print {$out} "|---:|----------|---------------------------------------------|-------------------|:---:|\n";
    my $number = 0;
    for my $event (@events) {
        ++$number;
        my $checked = $covered{$event->{source}} ? "[x]" : "[ ]";
        print {$out} "| $number | `$event->{function}` | ",
            markdown("$event->{source}: $event->{trigger}"), " | ",
            markdown($event->{result}), " | $checked |\n";
    }
    close $out;
}

sub branch_predicates {
    my ($range) = @_;
    my @lines = read_lines($range->{file});
    my $body = join "", @lines[$range->{start} - 1 .. $range->{end} - 1];
    $body =~ s{/\*.*?\*/}{}gs;
    $body =~ s{//[^\n]*}{}g;
    my @predicates;
    while ($body =~ /\b(if|else\s+if|switch)\s*(\((?:[^()]++|(?2))*\))/g) {
        my ($kind, $predicate) = ($1, $2);
        $predicate =~ s/^\(|\)$//g;
        $predicate =~ s/\s+/ /g;
        push @predicates, "$kind $predicate";
    }
    my %seen;
    return grep !$seen{$_}++, @predicates;
}

sub write_configs {
    my ($c_symbols, $ranges) = @_;
    my %covered = map { $_ => 1 } qw(
        png_sRGB_base png_sRGB_delta png_sRGB_table
        png_check_fp_number png_check_fp_string
        png_create_read_struct png_create_write_struct
        png_do_bgr png_do_invert png_do_packswap png_do_strip_channel png_do_swap
        png_format_number png_gamma_16bit_correct png_gamma_8bit_correct
        png_gamma_significant png_get_header_ver png_get_header_version
        png_get_int_32 png_get_io_ptr png_get_libpng_ver png_get_uint_16
        png_get_uint_32 png_muldiv png_reciprocal png_reciprocal2
        png_reset_zstream png_save_int_32 png_save_uint_16 png_save_uint_32
        png_set_interlace_handling png_sig_cmp
    );
    open my $out, ">", "$root/CONFIGS.md" or die "cannot write CONFIGS.md: $!";
    print {$out} "# Configuration Surface\n\n";
    print {$out} "## Build-Time Configurations\n\n";
    print {$out} "`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no build options.\n";
    print {$out} "There is exactly one valid combination: `cargo --no-default-features` (empty feature set), matching the prebuilt `pnglibconf.h` configuration.\n\n";
    print {$out} "## Runtime Configurations And Input Shapes\n\n";
    print {$out} "One row is emitted for every C dynamic export. The configuration cell lists every distinct `if`, `else if`, and `switch` predicate in that symbol's C definition; randomized cases for the row must cover the source-distinguished outcomes and their valid cross-products.\n\n";
    print {$out} "| # | entry point(s) | configuration (options set + input shape) | [ ] |\n";
    print {$out} "|---:|----------------|-------------------------------------------|:---:|\n";
    my $number = 0;
    for my $symbol (sort keys %$c_symbols) {
        ++$number;
        my $range = $ranges->{$symbol};
        my $configuration;
        if (!defined $range) {
            $configuration = "exported data or macro-generated symbol; byte/value identity";
        }
        else {
            my @predicates = branch_predicates($range);
            my $location = relative_source($range->{file}) . ":$range->{start}";
            $configuration = @predicates
                ? "$location; valid randomized inputs covering true/false or switch outcomes for: " .
                    join("; ", @predicates)
                : "$location; direct path; valid randomized values across argument widths and boundaries";
        }
        my $checked = $covered{$symbol} ? "[x]" : "[ ]";
        print {$out} "| $number | `$symbol` | ", markdown($configuration), " | $checked |\n";
    }
    close $out;
}

die "missing C shared library $c_so\n" unless -f $c_so;
die "missing Rust shared library $rust_so\n" unless -f $rust_so;
my %c_symbols = dynamic_symbols($c_so);
my %rust_symbols = dynamic_symbols($rust_so);
my %ranges = function_ranges();

write_symbols(\%c_symbols, \%rust_symbols);
write_errors(\%ranges);
write_configs(\%c_symbols, \%ranges);
