#!/usr/bin/env perl
use strict;
use warnings;
use File::Basename qw(basename);
use File::Spec;
use Cwd qw(abs_path);

my $crate = abs_path(File::Spec->catdir(File::Spec->curdir()));
my $c_root = abs_path(File::Spec->catdir($crate, '..', 'c_src'));
my $c_so = File::Spec->catfile($c_root, 'build', 'libpng.so');
my $rust_so = File::Spec->catfile($crate, 'target', 'release', 'liblibpng.so');

die "run from the translation crate root\n" unless -f 'Cargo.toml';
die "missing $c_so\n" unless -f $c_so;
die "missing $rust_so\n" unless -f $rust_so;

sub dynamic_symbols {
    my ($library) = @_;
    open my $nm, '-|', 'nm', '-D', '--defined-only', '--format=posix', $library
        or die "cannot run nm: $!\n";
    my %symbols;
    while (<$nm>) {
        my ($name, $kind) = split;
        $symbols{$name} = $kind if defined($name) && $name =~ /^png_/;
    }
    close $nm or die "nm failed for $library\n";
    return \%symbols;
}

sub markdown_escape {
    my ($text) = @_;
    $text //= '';
    $text =~ s/\s+/ /g;
    $text =~ s/^\s+|\s+$//g;
    $text =~ s/\|/\\|/g;
    $text =~ s/`/'/g;
    return $text;
}

sub source_files {
    return sort glob(File::Spec->catfile($c_root, 'src', '*.c'));
}

my %definitions;
my @errors;

for my $path (source_files()) {
    open my $source, '<', $path or die "cannot read $path: $!\n";
    my @lines = <$source>;
    close $source;
    chomp @lines;

    my $function = '(file scope)';
    my @guards;
    my $in_comment = 0;
    for my $index (0 .. $#lines) {
        my $line = $lines[$index];
        my $code = '';
        my $remaining = $line;
        while (length $remaining) {
            if ($in_comment) {
                if ($remaining =~ s/^.*?\*\///) {
                    $in_comment = 0;
                }
                else {
                    $remaining = '';
                }
            }
            elsif ($remaining =~ s/^(.*?)\/\*//) {
                $code .= $1;
                $in_comment = 1;
            }
            else {
                $code .= $remaining;
                $remaining = '';
            }
        }
        $code =~ s{//.*$}{};

        if ($code =~ /^([A-Za-z_][A-Za-z0-9_]*)\s*(?:,?\s*|\)\s*)\(/) {
            $function = $1;
            $definitions{$function} //= [basename($path), $index + 1]
                if $function =~ /^png_/;
            @guards = ();
        }

        if ($code =~ /^\s*(?:else\s+)?if\s*\((.*)/ ||
            $code =~ /^\s*switch\s*\((.*)/ ||
            $code =~ /^\s*case\s+(.+)/) {
            my $guard = $1;
            $guard =~ s/\s*\{\s*$//;
            push @guards, [$index + 1, $guard];
            shift @guards while @guards > 8;
        }

        my ($kind, $expected);
        my $is_definition = $code =~ /^[A-Za-z_][A-Za-z0-9_]*\s*(?:,?\s*|\)\s*)\(/;
        if (!$is_definition &&
            $code =~ /\b(png_(?:(?:chunk_)?error|fixed_error|default_error))\s*\(/) {
            ($kind, $expected) = ($1, 'fatal error callback, then longjmp/abort');
        }
        elsif (!$is_definition &&
            $code =~ /\b(png_(?:app_error|benign_error|chunk_benign_error|chunk_report|icc_profile_error))\s*\(/) {
            ($kind, $expected) = ($1, 'error or warning according to benign-error/chunk policy');
        }
        elsif (!$is_definition && $code =~ /\b(png_image_error)\s*\(/) {
            ($kind, $expected) = ($1, '0; image error status and message are set');
        }
        elsif (!$is_definition &&
            $code =~ /\b(png_(?:warning|chunk_warning|app_warning|safe_warning|formatted_warning|default_warning|warning_parameter(?:_signed|_unsigned)?))\s*\(/) {
            ($kind, $expected) = ($1, 'warning callback; input/chunk is ignored, clamped, or continued as coded');
        }
        elsif (!$is_definition && $code =~ /\b(png_crc_error)\s*\(/) {
            ($kind, $expected) = ($1, 'CRC policy result indicating whether the chunk must be rejected');
        }
        elsif (!$is_definition && $code =~ /\b(png_malloc_warn)\s*\(/) {
            ($kind, $expected) = ($1, 'allocation result or NULL without a fatal error');
        }
        elsif ($code =~ /\breturn\s+NULL\s*;/) {
            ($kind, $expected) = ('return NULL', 'NULL');
        }
        elsif ($code =~ /\breturn\s+-1\s*;/) {
            ($kind, $expected) = ('return -1', '-1');
        }
        elsif ($code =~ /\breturn\s+0\s*;/) {
            ($kind, $expected) = ('return 0', '0');
        }
        elsif ($code =~ /\breturn\s+(PNG_[A-Za-z0-9_]*(?:ERROR|INVALID|FAIL)[A-Za-z0-9_]*)\s*;/) {
            ($kind, $expected) = ('error enum', $1);
        }
        elsif ($code =~ /\b(?:PNG_)?assert\s*\(/i) {
            ($kind, $expected) = ('assert', 'assertion failure in assertion-enabled builds');
        }
        next unless defined $kind;

        my $trigger = @guards
            ? "guard near line $guards[-1][0]: $guards[-1][1]"
            : "unconditional at this source site";
        my $site = markdown_escape($code);
        push @errors, {
            file => basename($path),
            line => $index + 1,
            function => $function,
            trigger => markdown_escape("$trigger; site: $site"),
            expected => $expected,
            kind => $kind,
        };
    }
}

my $c_symbols = dynamic_symbols($c_so);
my $rust_symbols = dynamic_symbols($rust_so);
my @symbols = sort keys %$c_symbols;

open my $symbols_md, '>', 'SYMBOLS.md' or die "cannot write SYMBOLS.md: $!\n";
print {$symbols_md} "# Dynamic Symbol Surface\n\n";
print {$symbols_md} "Generated by `tools/generate_surfaces.pl` from `nm -D --defined-only`.\n\n";
print {$symbols_md} "- C symbols: ", scalar(@symbols), "\n";
print {$symbols_md} "- Rust symbols: ", scalar(keys %$rust_symbols), "\n";
my @missing = grep { !exists $rust_symbols->{$_} } @symbols;
print {$symbols_md} "- Missing from Rust: ", scalar(@missing), "\n\n";
print {$symbols_md} "| # | C symbol | kind | Rust export |\n";
print {$symbols_md} "|---:|----------|:----:|:-----------:|\n";
for my $index (0 .. $#symbols) {
    my $name = $symbols[$index];
    my $status = exists $rust_symbols->{$name} ? '[x]' : '[ ]';
    print {$symbols_md} '| ', $index + 1, " | `$name` | `$c_symbols->{$name}` | $status |\n";
}
close $symbols_md;

open my $errors_md, '>', 'ERRORS.md' or die "cannot write ERRORS.md: $!\n";
print {$errors_md} "# Error Surface\n\n";
print {$errors_md} "Mechanically extracted from executable C error/warning/policy calls, explicit `NULL`/`-1`/`0` returns, error enums, and assertions. ";
print {$errors_md} "The trigger column preserves the nearest lexical guard plus the exact source site; ";
print {$errors_md} "multiline guards should be read at the cited source location.\n\n";
print {$errors_md} "| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |\n";
print {$errors_md} "|---:|----------|---------------------------------------------|-------------------|:---:|\n";
for my $index (0 .. $#errors) {
    my $error = $errors[$index];
    print {$errors_md} '| ', $index + 1, " | `$error->{function}` | ";
    print {$errors_md} "`$error->{file}:$error->{line}` $error->{trigger} | ";
    print {$errors_md} "$error->{expected} | [ ] |\n";
}
close $errors_md;

open my $configs_md, '>', 'CONFIGS.md' or die "cannot write CONFIGS.md: $!\n";
print {$configs_md} "# Configuration Surface\n\n";
print {$configs_md} "Cargo declares no features, so the complete Cargo feature powerset is the default build ";
print {$configs_md} "and the equivalent `--no-default-features` build. The C feature configuration is fixed by ";
print {$configs_md} "`c_src/include/pnglibconf.h`.\n\n";
print {$configs_md} "One row per dynamic C entry point/table. Function rows are derived from `nm -D` and ";
print {$configs_md} "the defining C source. Tests exercise branch-relevant shape classes with fixed-seed inputs: ";
print {$configs_md} "empty/one/many, minimum/maximum/one-past scalars, valid/NULL pointers where the C checks them, ";
print {$configs_md} "all documented enum modes, and direct plus composed read/write paths.\n\n";
print {$configs_md} "| # | entry point(s) | configuration (options set + input shape) | [ ] |\n";
print {$configs_md} "|---:|----------------|-------------------------------------------|:---:|\n";
for my $index (0 .. $#symbols) {
    my $name = $symbols[$index];
    my $kind = $c_symbols->{$name};
    my $location = exists $definitions{$name}
        ? "$definitions{$name}[0]:$definitions{$name}[1]"
        : 'png.c static table';
    my $configuration = $kind eq 'T'
        ? "C-defined ABI function at $location; every source-selected option/mode and reachable input-shape branch"
        : "read-only data table at $location; full byte extent and alignment";
    print {$configs_md} '| ', $index + 1, " | `$name` | $configuration | [ ] |\n";
}
close $configs_md;

print "symbols=", scalar(@symbols),
      " missing=", scalar(@missing),
      " errors=", scalar(@errors),
      " configs=", scalar(@symbols), "\n";
