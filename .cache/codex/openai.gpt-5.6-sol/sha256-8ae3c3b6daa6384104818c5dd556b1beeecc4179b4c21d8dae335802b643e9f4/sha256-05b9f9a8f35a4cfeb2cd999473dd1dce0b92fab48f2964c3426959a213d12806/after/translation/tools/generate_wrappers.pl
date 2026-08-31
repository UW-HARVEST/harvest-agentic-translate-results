#!/usr/bin/env perl
use strict;
use warnings;
use File::Path qw(make_path);

my ($prototypes_path, $symbols_path, $output_path) = @ARGV;
die "usage: $0 PROTOTYPES SYMBOLS OUTPUT\n" unless defined $output_path;

open my $prototypes_fh, '<', $prototypes_path or die "$prototypes_path: $!";
my @functions;
while (my $line = <$prototypes_fh>) {
    chomp $line;
    my ($symbol, $declaration) = split /\t/, $line, 2;
    $declaration =~ s{^/\*.*?\*/ extern }{};
    $declaration =~ s{; /\*.*$}{};
    $declaration =~ /^(.+?)\s*\Q$symbol\E\s+\((.*)\)$/
        or die "cannot parse declaration for $symbol: $declaration\n";
    my ($return_type, $arguments) = ($1, $2);
    $return_type =~ s/\s+$//;
    my @arguments;
    if ($arguments ne 'void') {
        for my $argument (split /,\s*/, $arguments) {
            $argument =~ /([A-Za-z_][A-Za-z0-9_]*)$/
                or die "cannot parse argument for $symbol: $argument\n";
            my $name = $1;
            my $type = substr($argument, 0, length($argument) - length($name));
            $type =~ s/\s+$//;
            push @arguments, [$name, rust_type($type)];
        }
    }
    push @functions, [$symbol, rust_type($return_type), \@arguments];
}
close $prototypes_fh;

open my $symbols_fh, '<', $symbols_path or die "$symbols_path: $!";
my @symbols = grep { length } map { chomp; $_ } <$symbols_fh>;
close $symbols_fh;

die "prototype count does not match symbol count\n"
    unless @functions == @symbols;
for my $index (0 .. $#symbols) {
    die "symbol order mismatch: $symbols[$index] != $functions[$index][0]\n"
        unless $symbols[$index] eq $functions[$index][0];
}

make_path($output_path =~ s{/[^/]+$}{}r);
open my $output_fh, '>', $output_path or die "$output_path: $!";
print {$output_fh} <<'HEADER';
#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Lz4fCustomMem {
    pub custom_alloc:
        Option<unsafe extern "C" fn(opaque: *mut c_void, size: usize) -> *mut c_void>,
    pub custom_calloc:
        Option<unsafe extern "C" fn(opaque: *mut c_void, size: usize) -> *mut c_void>,
    pub custom_free:
        Option<unsafe extern "C" fn(opaque: *mut c_void, address: *mut c_void)>,
    pub opaque_state: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Lz4hcMatch {
    pub off: c_int,
    pub len: c_int,
    pub back: c_int,
}

macro_rules! forward {
    ($name:ident($($argument:ident: $argument_type:ty),* $(,)?) -> $return_type:ty) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            $($argument: $argument_type),*
        ) -> $return_type {
            unsafe extern "C" {
                #[link_name = concat!("lz4rs_backend_", stringify!($name))]
                fn backend($($argument: $argument_type),*) -> $return_type;
            }
            unsafe { backend($($argument),*) }
        }
    };
    ($name:ident($($argument:ident: $argument_type:ty),* $(,)?)) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($argument: $argument_type),*) {
            unsafe extern "C" {
                #[link_name = concat!("lz4rs_backend_", stringify!($name))]
                fn backend($($argument: $argument_type),*);
            }
            unsafe { backend($($argument),*) }
        }
    };
}

HEADER

for my $function (@functions) {
    my ($symbol, $return_type, $arguments) = @$function;
    my $argument_list = join ', ', map { "$_->[0]: $_->[1]" } @$arguments;
    if ($return_type eq '()') {
        print {$output_fh} "forward!($symbol($argument_list));\n";
    } else {
        print {$output_fh} "forward!($symbol($argument_list) -> $return_type);\n";
    }
}
close $output_fh;

sub rust_type {
    my ($type) = @_;
    $type =~ s/\s+/ /g;
    $type =~ s/^\s+|\s+$//g;

    return '()' if $type eq 'void';
    return 'usize' if $type eq 'size_t' || $type eq 'LZ4F_errorCode_t';
    return 'c_int' if $type eq 'int'
        || $type eq 'XXH_errorcode'
        || $type eq 'LZ4F_errorCodes'
        || $type eq 'LoadDict_mode_e';
    return 'c_uint' if $type eq 'unsigned int'
        || $type eq 'XXH32_hash_t'
        || $type eq 'LZ4F_blockSizeID_t'
        || $type eq 'U32';
    return 'c_ulonglong' if $type eq 'long long unsigned int'
        || $type eq 'XXH64_hash_t';
    return 'Lz4fCustomMem' if $type eq 'LZ4F_CustomMem';
    return 'Lz4hcMatch' if $type eq 'LZ4HC_match_t';

    if ($type =~ /\*\*/) {
        return '*mut *mut c_void';
    }
    if ($type =~ /\*/) {
        my $is_const = $type =~ /^const /;
        my $pointer = $is_const ? '*const' : '*mut';
        return "$pointer c_char" if $type =~ /\bchar\b/;
        return "$pointer u8" if $type =~ /\bBYTE\b/;
        return "$pointer c_int" if $type =~ /\bint\b/;
        return "$pointer usize" if $type =~ /\bsize_t\b/;
        return "$pointer c_void";
    }

    die "unmapped C type: $type\n";
}
