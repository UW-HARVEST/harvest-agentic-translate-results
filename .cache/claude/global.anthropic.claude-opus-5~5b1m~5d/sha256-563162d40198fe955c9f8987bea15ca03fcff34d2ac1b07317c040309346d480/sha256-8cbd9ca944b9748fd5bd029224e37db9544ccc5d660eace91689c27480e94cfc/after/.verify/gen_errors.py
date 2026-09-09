import csv
rows=[r for r in csv.reader(open('.verify/errsites.tsv'), delimiter='\t')]
kindres={
 'png_error':'FATAL: error_fn(msg) then longjmp/abort — never returns',
 'png_chunk_error':'FATAL: error_fn("<chunkname>: msg") then longjmp',
 'png_fixed_error':'FATAL: error_fn("fixed point overflow in <s>")',
 'png_longjmp':'FATAL: longjmp_fn(jmp_buf, val) / PNG_ABORT',
 'png_benign_error':'FATAL if flags&BENIGN_*_ERRORS_WARN==0 else warning_fn(msg)',
 'png_app_error':'FATAL unless flags&APP_WARNINGS_WARN -> warning_fn(msg)',
 'png_warning':'warning_fn(msg), returns normally',
 'png_app_warning':'warning_fn(msg), returns normally',
 'png_chunk_warning':'warning_fn("<chunkname>: msg")',
 'png_chunk_report':'PNG_CHUNK_ERROR->png_chunk_error / WARNING->png_chunk_warning / benign',
}
out=[]
out.append('# ERRORS.md — error-surface table (derived mechanically from `c_src/src/*.c`)\n')
out.append("""
Every row is ONE distinct rejection site in the C source, found with:

```
grep -nE '\\b(png_error|png_chunk_error|png_benign_error|png_app_error|png_warning|png_app_warning|png_chunk_warning|png_chunk_report|png_fixed_error|png_longjmp)[ ]*\\(' c_src/src/*.c
```

392 sites total.  There are **no `assert`s** anywhere in the library
(`grep -rn assert c_src/src` finds only comments), so every rejection is one of
these calls or a sentinel return (see the sentinel section at the end).

How the "expected C result" is observed in the differential tests: both libraries get an
`error_fn`/`warning_fn` installed with `png_set_error_fn`, which records the exact message
bytes; the error callback then unwinds (panic, standing in for the app's `longjmp`).  A test
asserts C and Rust produce (a) the same sequence of recorded (kind, message) pairs and
(b) the same return value / `png_image.warning_or_error` + `png_image.message`.

Legend for "expected C result": see the table right below; `msg` is the message column.

| macro | expected C behaviour |
|---|---|
""")
for k,v in kindres.items(): out.append(f'| `{k}` | {v} |\n')
out.append('\n## Rejection sites\n\n| # | function (file:line) | macro | trigger (guard in the C source) | message / expected C result | test |\n|---|---|---|---|---|---|\n')
for i,(f,ln,fn,kind,msg,guard) in enumerate(rows,1):
    g=guard.replace('|','\\|').strip() or '(unconditional at that point)'
    m=msg.replace('|','\\|')
    out.append(f'| {i} | `{fn}` ({f}:{ln}) | `{kind}` | `{g}` | "{m}" | [ ] |\n')
open('translation/ERRORS.md','w').writelines(out)
print(len(rows))
