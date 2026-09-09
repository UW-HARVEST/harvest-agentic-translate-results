#!/usr/bin/env perl
use strict;
use warnings;
use File::Basename qw(dirname);
use File::Find qw(find);
use File::Spec;

my $crate = File::Spec->rel2abs(dirname(dirname(__FILE__)));
my $root = File::Spec->rel2abs(File::Spec->catdir($crate, ".."));
my $c_root = File::Spec->catdir($root, "c_src");
my $c_so = File::Spec->catfile($c_root, "build", "libjansson.so");
my $rust_so = File::Spec->catfile($crate, "target", "release", "libjansson.so");

sub symbols {
    my ($path) = @_;
    open my $nm, "-|", "nm", "-D", "--defined-only", $path
        or die "cannot run nm for $path: $!";
    my @symbols;
    while (<$nm>) {
        chomp;
        my @fields = split;
        push @symbols, $fields[-1] if @fields;
    }
    close $nm or die "nm failed for $path";
    my %seen;
    return sort grep { !$seen{$_}++ } @symbols;
}

my @c_symbols = symbols($c_so);
my %rust_symbols = map { $_ => 1 } symbols($rust_so);

open my $symbols_md, ">", File::Spec->catfile($crate, "SYMBOLS.md")
    or die "cannot write SYMBOLS.md: $!";
print {$symbols_md} "# Dynamic symbol surface\n\n";
print {$symbols_md} "Generated mechanically from `nm -D --defined-only` on both shared libraries.\n\n";
print {$symbols_md} "| C symbol | Rust export |\n";
print {$symbols_md} "|----------|-------------|\n";
for my $symbol (@c_symbols) {
    my $status = $rust_symbols{$symbol} ? "[x]" : "[ ] MISSING";
    print {$symbols_md} "| `$symbol` | $status |\n";
}
my @missing = grep { !$rust_symbols{$_} } @c_symbols;
print {$symbols_md} "\nMissing symbols: **", scalar(@missing), "**\n";
close $symbols_md or die "cannot close SYMBOLS.md: $!";

my @sources;
find(
    sub {
        push @sources, $File::Find::name if -f $_ && /\.(?:c|h)$/;
    },
    File::Spec->catdir($c_root, "src"),
    File::Spec->catdir($c_root, "include"),
);
@sources = sort @sources;

my @errors;
for my $path (@sources) {
    open my $fh, "<", $path or die "cannot read $path: $!";
    my @lines = <$fh>;
    close $fh;

    my $relative = File::Spec->abs2rel($path, $root);
    my @functions;
    my $signature = "";
    my $signature_start = 0;
    my $depth = 0;
    for my $i (0 .. $#lines) {
        my $line = $lines[$i];
        if ($depth == 0) {
            if ($signature eq "" && $line =~ /^\s*(?:static\s+)?[A-Za-z_]/) {
                $signature = $line;
                $signature_start = $i + 1;
            } elsif ($signature ne "") {
                $signature .= $line;
            }
            if ($signature ne "" && $signature =~ /\{/) {
                if ($signature =~ /([A-Za-z_][A-Za-z0-9_]*)\s*\([^;{}]*\)\s*\{/s) {
                    my $name = $1;
                    if ($name !~ /^(?:if|for|while|switch)$/) {
                        push @functions, [$signature_start, $name];
                    }
                }
                $signature = "";
            } elsif ($signature ne "" && $signature =~ /;/) {
                $signature = "";
            } elsif ($signature ne "" && length($signature) > 1200) {
                $signature = "";
            }
        }
        my $opens = () = $line =~ /\{/g;
        my $closes = () = $line =~ /\}/g;
        $depth += $opens - $closes;
        $depth = 0 if $depth < 0;
    }

    for my $i (0 .. $#lines) {
        my $line = $lines[$i];
        next unless $line =~ /
            \bassert\s*\(
          | \bRETURN_ERROR\b
          | \breturn\s+(?:-1|NULL)\s*;
        /x;

        my $function = "(file scope)";
        for my $entry (@functions) {
            last if $entry->[0] > $i + 1;
            $function = $entry->[1];
        }

        my @context;
        my $start = $i > 5 ? $i - 5 : 0;
        for my $j ($start .. $i) {
            my $text = $lines[$j];
            $text =~ s/^\s+|\s+$//g;
            next if $text eq "" || $text =~ m{^(?:/\*|\*|//)};
            push @context, $text;
        }
        my $trigger = join(" ", @context);
        $trigger =~ s/\|/\\|/g;
        $trigger =~ s/`/'/g;
        $trigger =~ s/\s+/ /g;

        my $expected =
            $line =~ /\bassert\s*\(/ ? "process aborts when assertion is enabled"
          : $line =~ /\breturn\s+-1/ ? "returns `-1`"
          : $line =~ /\breturn\s+NULL/ ? "returns `NULL`"
          : "returns the macro's error result";
        push @errors, [$function, "$trigger (`$relative:" . ($i + 1) . "`)", $expected];
    }
}

open my $errors_md, ">", File::Spec->catfile($crate, "ERRORS.md")
    or die "cannot write ERRORS.md: $!";
print {$errors_md} "# Error surface\n\n";
print {$errors_md} "Generated mechanically from every `RETURN_ERROR`, literal `return -1`, literal `return NULL`, and `assert(...)` in the C source. The trigger column preserves the local source branch verbatim and records its exact location.\n\n";
print {$errors_md} "| # | function | trigger (the exact invalid input/condition) | expected C result | tested |\n";
print {$errors_md} "|---|----------|---------------------------------------------|-------------------|--------|\n";
my $number = 0;
for my $row (@errors) {
    $number++;
    print {$errors_md} "| $number | `$row->[0]` | $row->[1] | $row->[2] | [ ] |\n";
}
close $errors_md or die "cannot close ERRORS.md: $!";

print "symbols=", scalar(@c_symbols), " missing=", scalar(@missing),
      " errors=", scalar(@errors), "\n";
