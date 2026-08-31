// Level 8: structural equivalence of the builtin object graph, property-tree
// ordering, and behaviour at the engine's hard limits.
mod common;

use common::*;

fn pair() -> (Session, Session) {
    (Session::new(Side::C, 0), Session::new(Side::Rust, 0))
}

fn same(cs: &Session, rs: &Session, label: &str, src: &str) {
    let a = run_script(cs, src);
    let b = run_script(rs, src);
    assert_eq!(a, b, "{}", label);
}

/// Walk the whole reachable builtin graph and dump names + property attributes.
const DUMP_GRAPH: &str = r#"
var seen = [];
var out = [];
function id(o) {
  for (var i = 0; i < seen.length; ++i) if (seen[i] === o) return i;
  seen.push(o);
  return seen.length - 1;
}
function attrs(o, k) {
  var d = Object.getOwnPropertyDescriptor(o, k);
  if (!d) return '?';
  var s = '';
  s += d.writable ? 'w' : '-';
  s += d.enumerable ? 'e' : '-';
  s += d.configurable ? 'c' : '-';
  if ('get' in d || 'set' in d) s += (d.get ? 'G' : '-') + (d.set ? 'S' : '-');
  return s;
}
function kind(v) {
  var t = typeof v;
  if (t !== 'object' && t !== 'function') return t;
  if (v === null) return 'null';
  return Object.prototype.toString.call(v);
}
function walk(o, path, depth) {
  if (depth > 4) return;
  if (o === null || (typeof o !== 'object' && typeof o !== 'function')) return;
  var i = id(o);
  var names = Object.getOwnPropertyNames(o).sort();
  out.push(path + ' #' + i + ' [' + names.join(',') + ']');
  for (var n = 0; n < names.length; ++n) {
    var k = names[n];
    var v;
    try { v = o[k]; } catch (e) { out.push(path + '.' + k + ' THROWS ' + e.name); continue; }
    out.push(path + '.' + k + ' ' + attrs(o, k) + ' ' + kind(v) +
             (typeof v === 'function' ? '/' + v.length + '/' + v.name : ''));
    if (v !== o && (typeof v === 'object' || typeof v === 'function')) {
      var already = false;
      for (var q = 0; q < seen.length; ++q) if (seen[q] === v) { already = true; break; }
      if (!already) walk(v, path + '.' + k, depth + 1);
      else out.push(path + '.' + k + ' -> #' + id(v));
    }
  }
}
walk(this, 'global', 0);
out.join('\n');
"#;

#[test]
fn builtin_object_graph_identical() {
    let (cs, rs) = pair();
    let a = run_script(&cs, DUMP_GRAPH);
    let b = run_script(&rs, DUMP_GRAPH);
    // Compare line by line for a readable first difference.
    let (sa, sb) = (a.value.clone().unwrap_or_default(), b.value.clone().unwrap_or_default());
    assert!(sa.len() > 2000, "graph dump suspiciously small: {}", sa.len());
    if sa != sb {
        let la: Vec<&str> = sa.split("\\n").collect();
        let lb: Vec<&str> = sb.split("\\n").collect();
        for (i, (x, y)) in la.iter().zip(lb.iter()).enumerate() {
            assert_eq!(x, y, "builtin graph differs at line {}", i);
        }
        assert_eq!(la.len(), lb.len(), "builtin graph line count differs");
    }
    assert_eq!(a, b, "builtin graph dump differs");
}

#[test]
fn prototype_identity_matrix() {
    let (cs, rs) = pair();
    let src = r#"
var names = ['Object','Array','Function','String','Number','Boolean','Date','RegExp',
             'Error','EvalError','RangeError','ReferenceError','SyntaxError','TypeError',
             'URIError','Math','JSON'];
var out = [];
for (var i = 0; i < names.length; ++i) {
  var g = this[names[i]];
  out.push(names[i] + ' typeof=' + typeof g);
  if (g === undefined) continue;
  out.push('  proto-of-ctor-is-Function.prototype=' + (Object.getPrototypeOf(g) === Function.prototype));
  if (g.prototype !== undefined) {
    out.push('  has prototype=' + (typeof g.prototype));
    out.push('  prototype.constructor===g:' + (g.prototype.constructor === g));
    out.push('  proto-of-prototype-is-Object.prototype=' + (Object.getPrototypeOf(g.prototype) === Object.prototype));
  }
  out.push('  length=' + g.length + ' name=' + g.name);
}
out.push('Object.getPrototypeOf(Object.prototype)=' + Object.getPrototypeOf(Object.prototype));
out.join('|');
"#;
    same(&cs, &rs, "prototype matrix", src);
}

#[test]
fn property_tree_ordering_stress() {
    let (cs, rs) = pair();
    // Enumeration order depends on the internal red-black tree shape, so this
    // is a strong structural check of jsproperty.c.
    let patterns: Vec<String> = vec![
        // ascending
        "var o={}; for(var i=0;i<200;i++) o['k'+i]=i; Object.keys(o).join(',')".into(),
        // descending
        "var o={}; for(var i=200;i>0;i--) o['k'+i]=i; Object.keys(o).join(',')".into(),
        // zig-zag
        "var o={}; for(var i=0;i<100;i++){o['a'+i]=i; o['z'+(100-i)]=i;} Object.keys(o).join(',')".into(),
        // insert then delete every other
        "var o={}; for(var i=0;i<200;i++) o['k'+i]=i; for(var i=0;i<200;i+=2) delete o['k'+i]; Object.keys(o).join(',')".into(),
        // delete from the front
        "var o={}; for(var i=0;i<100;i++) o['k'+i]=i; for(var i=0;i<50;i++) delete o['k'+i]; Object.keys(o).join(',')".into(),
        // delete from the back
        "var o={}; for(var i=0;i<100;i++) o['k'+i]=i; for(var i=99;i>=50;i--) delete o['k'+i]; Object.keys(o).join(',')".into(),
        // re-insert after deleting
        "var o={}; for(var i=0;i<100;i++) o['k'+i]=i; for(var i=0;i<100;i+=3) delete o['k'+i]; for(var i=0;i<100;i+=3) o['k'+i]=i; Object.keys(o).join(',')".into(),
        // single character keys, all printable ASCII
        "var o={}; for(var i=32;i<127;i++) o[String.fromCharCode(i)]=i; Object.keys(o).join('')".into(),
        // numeric-looking keys
        "var o={}; for(var i=0;i<100;i++) o[i]=i; Object.keys(o).join(',')".into(),
        "var o={}; for(var i=99;i>=0;i--) o[i]=i; Object.keys(o).join(',')".into(),
        // mixed numeric/string
        "var o={}; for(var i=0;i<50;i++){o[i]=i; o['s'+i]=i;} Object.keys(o).join(',')".into(),
        // long shared prefixes
        "var o={}; for(var i=0;i<100;i++) o['prefix_prefix_prefix_'+i]=i; Object.keys(o).length+':'+Object.keys(o)[0]".into(),
        // for-in on the same shapes
        "var o={}; for(var i=0;i<150;i++) o['k'+i]=i; var s=''; for(var k in o) s+=k+' '; s".into(),
        "var o={}; for(var i=0;i<150;i++) o['k'+i]=i; for(var i=0;i<150;i+=2) delete o['k'+i]; var s=''; for(var k in o) s+=k+' '; s".into(),
    ];
    for (i, p) in patterns.iter().enumerate() {
        same(&cs, &rs, &format!("property tree pattern {}", i), p);
    }

    // Pseudo-random insert/delete sequences.
    let mut x: u64 = 0x51ED;
    for round in 0..20 {
        let mut prog = String::from("var o={}; var log=[];");
        for _ in 0..120 {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let key = (x >> 33) % 60;
            if (x >> 20) % 3 == 0 {
                prog.push_str(&format!("delete o['k{}'];", key));
            } else {
                prog.push_str(&format!("o['k{}']={};", key, key));
            }
        }
        prog.push_str("Object.keys(o).join(',')+'|'+Object.getOwnPropertyNames(o).join(',')");
        same(&cs, &rs, &format!("random property tree round {}", round), &prog);
    }
}

#[test]
fn array_storage_transitions() {
    let (cs, rs) = pair();
    let cases = [
        "var a=[]; for(var i=0;i<100;i++) a[i]=i; a.length+':'+a.join(',')",
        "var a=[]; for(var i=99;i>=0;i--) a[i]=i; a.length+':'+a.join(',')",
        "var a=[]; a[0]=1; a[1000]=2; a.length+':'+Object.keys(a).join(',')",
        "var a=[1,2,3]; a[10]=4; a.length+':'+Object.keys(a).join(',')",
        "var a=[1,2,3]; delete a[1]; a[1]=9; a.join(',')",
        "var a=[]; for(var i=0;i<50;i++) a.push(i); for(var i=0;i<25;i++) a.shift(); a.join(',')",
        "var a=[]; for(var i=0;i<50;i++) a.unshift(i); a.join(',')",
        "var a=[1,2,3]; a.length=0; a.length+':'+a.join(',')",
        "var a=[1,2,3]; a.length=100; a.length+':'+Object.keys(a).join(',')",
        "var a=[1,2,3]; a.length=100; a.length=2; a.join(',')",
        "var a=[]; a.length=5; a[2]='x'; a.join(',')+':'+Object.keys(a).join(',')",
        "var a=[1,2,3]; a.foo='bar'; Object.keys(a).join(',')",
        "var a=[1,2,3]; a.foo='bar'; a.length=1; Object.keys(a).join(',')",
        "var a=[1,2,3]; a['1.0']=9; Object.keys(a).join(',')+':'+a.join(',')",
        "var a=[1,2,3]; a['-1']=9; Object.keys(a).join(',')",
        "var a=[1,2,3]; a['4294967295']=9; Object.keys(a).join(',')+':'+a.length",
        "var a=[1,2,3]; a['4294967294']=9; a.length",
        "var a=[1,2,3]; Object.defineProperty(a,'1',{value:9}); a.join(',')",
        "var a=[1,2,3]; Object.defineProperty(a,'length',{value:1}); a.join(',')",
        "var a=[3,1,2]; a.sort(); a.length=2; a.join(',')",
        "var a=[]; for(var i=0;i<1000;i++)a[i]=1000-i; a.sort(function(x,y){return x-y}); a[0]+','+a[999]",
        "var a=[]; for(var i=0;i<300;i++)a[i]=String(i); a.sort().slice(0,10).join(',')",
        "var a=[1,2,3,4,5]; a.splice(0,0,'a','b'); a.join(',')",
        "var a=[1,2,3,4,5]; a.splice(5,0,'a'); a.join(',')",
        "var a=[1,2,3,4,5]; a.splice(2,10); a.join(',')",
        "var a=[]; a[4294967294]='x'; a.length+':'+a[4294967294]",
        "var a=[]; try{a.length=-1}catch(e){e.name}",
        "var a=[]; try{a.length=1.5}catch(e){e.name}",
        "var a=[]; a.length=4294967295; a.length",
        "var a=[]; try{a.length=4294967296}catch(e){e.name}",
        "var a=[1,2,3]; var b=a.concat(a,a); b.length+':'+b.join('')",
        "var a=[1,2,3]; a.reverse(); a[10]=1; a.reverse(); a.join(',')",
    ];
    for (i, c) in cases.iter().enumerate() {
        same(&cs, &rs, &format!("array transition {}", i), c);
    }
}

#[test]
fn recursion_and_limit_behaviour() {
    let (cs, rs) = pair();
    let cases = [
        // deep JS recursion -> should hit the same limit
        "function f(n){return n<=0?0:1+f(n-1)} try{f(100)}catch(e){e.name}",
        "function f(n){return n<=0?0:1+f(n-1)} try{f(500)}catch(e){e.name}",
        "function f(n){return n<=0?0:1+f(n-1)} try{f(5000)}catch(e){e.name+':'+e.message}",
        "function f(){return f()} try{f()}catch(e){e.name+':'+e.message}",
        // deep try nesting
        "var s=''; function f(n){ if(n<=0) throw 'x'; try{ f(n-1) }catch(e){ throw e } } try{f(100)}catch(e){e}",
        // deeply nested data structure -> repr / JSON recursion
        "var o={}; var p=o; for(var i=0;i<200;i++){ p.next={}; p=p.next } try{JSON.stringify(o).length}catch(e){e.name+':'+e.message}",
        "var a=[]; var p=a; for(var i=0;i<200;i++){ var q=[]; p.push(q); p=q } try{JSON.stringify(a).length}catch(e){e.name+':'+e.message}",
        // cyclic structures
        "var o={}; o.o=o; try{JSON.stringify(o)}catch(e){e.name+':'+e.message}",
        "var a=[]; a.push(a); try{JSON.stringify(a)}catch(e){e.name+':'+e.message}",
        "var o={}; o.o=o; try{String(o)}catch(e){e.name}",
        // deeply nested expressions at parse time
        &format!("try{{ eval('{}1{}') }}catch(e){{ e.name }}", "(".repeat(300), ")".repeat(300)),
        &format!("try{{ eval('{}1{}') }}catch(e){{ e.name }}", "(".repeat(2000), ")".repeat(2000)),
        &format!("try{{ eval('{}1') }}catch(e){{ e.name }}", "!".repeat(3000)),
        // huge string building
        "var s='x'; try{ for(var i=0;i<24;i++) s+=s; s.length }catch(e){ e.name }",
        // many arguments
        &format!("function f(){{return arguments.length}} f({})", (0..500).map(|i| i.to_string()).collect::<Vec<_>>().join(",")),
        // deep with-nesting
        &format!("try{{ eval('{}1{}') }}catch(e){{ e.name }}", "with({}){".repeat(100), "}".repeat(100)),
        // deep function nesting
        &format!("try{{ eval('{}1{}') }}catch(e){{ e.name }}", "(function(){return ".repeat(150), "})()".repeat(150)),
    ];
    for (i, c) in cases.iter().enumerate() {
        same(&cs, &rs, &format!("recursion/limit case {}", i), c);
    }
}

#[test]
fn runlimit_and_memlimit_behaviour() {
    for (run, mem) in [(1i32, 0i32), (5, 0), (0, 64 * 1024), (0, 1), (2, 4096)] {
        let cs = Session::new(Side::C, 0);
        let rs = Session::new(Side::Rust, 0);
        unsafe { (cs.vm.setlimit)(cs.j, run, mem) };
        unsafe { (rs.vm.setlimit)(rs.j, run, mem) };
        for src in [
            "1+1",
            "function f(n){return n<=0?0:1+f(n-1)} try{f(20)}catch(e){e.name+':'+e.message}",
            "var a=[]; try{for(var i=0;i<5000;i++)a.push({x:i}); a.length}catch(e){e.name+':'+e.message}",
            "var s='x'; try{for(var i=0;i<18;i++)s+=s; s.length}catch(e){e.name+':'+e.message}",
            "try{var o={}; for(var i=0;i<5000;i++)o['k'+i]=i; Object.keys(o).length}catch(e){e.name+':'+e.message}",
        ] {
            let a = run_script(&cs, src);
            let b = run_script(&rs, src);
            assert_eq!(a, b, "setlimit({},{}) on {:?}", run, mem, src);
        }
    }
}

#[test]
fn regexp_engine_limits() {
    let (cs, rs) = pair();
    let mut cases: Vec<String> = Vec::new();
    // many alternations / groups / classes -> program size and sub limits
    for n in [1usize, 8, 15, 16, 17, 20, 40] {
        cases.push(format!(
            "try{{ String(new RegExp('{}')) }}catch(e){{ e.name+':'+e.message }}",
            "(a)".repeat(n)
        ));
        cases.push(format!(
            "try{{ String(new RegExp('{}').exec('{}')) }}catch(e){{ e.name+':'+e.message }}",
            "(a)".repeat(n),
            "a".repeat(n)
        ));
        cases.push(format!(
            "try{{ String(new RegExp('{}')) }}catch(e){{ e.name+':'+e.message }}",
            (0..n).map(|_| "[a-c]").collect::<Vec<_>>().join("|")
        ));
    }
    for n in [10usize, 100, 1000, 5000, 20000] {
        cases.push(format!(
            "try{{ String(new RegExp('{}').test('aaaa')) }}catch(e){{ e.name+':'+e.message }}",
            "a?".repeat(n.min(4000))
        ));
        cases.push(format!(
            "try{{ String(new RegExp('a{{{},{}}}').test('{}')) }}catch(e){{ e.name+':'+e.message }}",
            1,
            n,
            "a".repeat(20)
        ));
    }
    // deeply nested groups -> recursion limit in the compiler/matcher
    for n in [10usize, 100, 200, 400] {
        cases.push(format!(
            "try{{ String(new RegExp('{}a{}').test('a')) }}catch(e){{ e.name+':'+e.message }}",
            "(".repeat(n),
            ")".repeat(n)
        ));
    }
    // catastrophic backtracking guarded by REG_MAXREC
    cases.push("try{ String(/(a+)+b/.test('aaaaaaaaaaaaaaaaaaaaaaaaaaaa')) }catch(e){ e.name+':'+e.message }".into());
    cases.push("try{ String(/(a|aa)+b/.test('aaaaaaaaaaaaaaaaaaaa')) }catch(e){ e.name+':'+e.message }".into());
    cases.push("try{ String(/(x+x+)+y/.test('xxxxxxxxxxxxxxxxxxxx')) }catch(e){ e.name+':'+e.message }".into());
    // back references beyond the group count
    for n in 1..12 {
        cases.push(format!(
            "try{{ String(new RegExp('(a)\\\\{}').exec('aa')) }}catch(e){{ e.name+':'+e.message }}",
            n
        ));
    }
    // huge character classes
    cases.push(format!(
        "try{{ String(new RegExp('[{}]').test('a')) }}catch(e){{ e.name+':'+e.message }}",
        (0..300).map(|i| format!("\\\\u{:04x}", 0x100 + i)).collect::<Vec<_>>().join("")
    ));
    for (i, c) in cases.iter().enumerate() {
        same(&cs, &rs, &format!("regexp limit case {}", i), c);
    }
}

#[test]
fn string_limit_and_large_values() {
    let (cs, rs) = pair();
    let cases = [
        "var s='x'; for(var i=0;i<20;i++) s+=s; s.length",
        "try{ var s='x'; for(var i=0;i<30;i++) s+=s; s.length }catch(e){ e.name+':'+e.message }",
        "var a=[]; for(var i=0;i<20000;i++)a.push('y'); a.join('').length",
        "try{ new Array(1e9).join('x').length }catch(e){ e.name+':'+e.message }",
        "try{ '.'.repeat }catch(e){ e.name }",
        "var s=''; for(var i=0;i<1000;i++) s+=String.fromCharCode(i+32); s.length",
        "var s=''; for(var i=0;i<300;i++) s+=String.fromCharCode(0x4e00+i); s.length+':'+s.charCodeAt(299)",
        "JSON.stringify(new Array(1000).join('a')).length",
        "encodeURIComponent(new Array(500).join('\\u00e9')).length",
        "var s=new Array(3000).join('a'); s.indexOf('b')+':'+s.lastIndexOf('a')",
        "var s=new Array(2000).join('ab'); s.split('a').length",
        "var s=new Array(2000).join('ab'); s.replace(/a/g,'X').length",
        "var s=new Array(500).join('ab'); s.match(/ab/g).length",
    ];
    for (i, c) in cases.iter().enumerate() {
        same(&cs, &rs, &format!("string limit case {}", i), c);
    }
}

#[test]
fn getter_setter_and_error_interactions() {
    let (cs, rs) = pair();
    let cases = [
        "var o={get x(){throw new Error('g')}}; try{o.x}catch(e){e.message}",
        "var o={set x(v){throw new Error('s')}}; try{o.x=1}catch(e){e.message}",
        "var o={get x(){return this}}; o.x===o",
        "var o={get x(){delete this.x; return 1}}; o.x+':'+('x' in o)",
        "var o={}; Object.defineProperty(o,'x',{get:function(){return 1},enumerable:true}); JSON.stringify(o)",
        "var o={}; Object.defineProperty(o,'x',{get:function(){return 1}}); Object.keys(o).length",
        "var a=[]; Object.defineProperty(a,'0',{get:function(){return 'g'}}); a.length=1; a.join(',')",
        "var o={valueOf:function(){throw new Error('v')}}; try{o*1}catch(e){e.message}",
        "var o={toString:function(){throw new Error('t')}}; try{''+o}catch(e){e.message}",
        "var o={}; o.toString=null; try{''+o}catch(e){e.name}",
        "var o={}; o.valueOf=null; o.toString=function(){return 'ok'}; ''+o",
        "var a=[{toString:function(){throw new Error('j')}}]; try{a.join(',')}catch(e){e.message}",
        "try{[1,2,3].sort(function(){throw new Error('c')})}catch(e){e.message}",
        "try{[1,2,3].map(function(){throw new Error('m')})}catch(e){e.message}",
        "try{JSON.stringify({get a(){throw new Error('js')}})}catch(e){e.message}",
        "try{JSON.parse('1',function(){throw new Error('jp')})}catch(e){e.message}",
        "try{'abc'.replace(/b/,function(){throw new Error('r')})}catch(e){e.message}",
        "var n=0; var o={get x(){return ++n}}; o.x+','+o.x+','+o.x",
        "var o=Object.create({get x(){return 'proto'}}); o.x",
        "var o=Object.create({set x(v){this.got=v}}); o.x=5; String(o.got)+':'+('x' in o)",
        "var o={}; Object.defineProperty(o,'x',{get:function(){return 1},set:undefined}); o.x=2; o.x",
        "'use strict'; var o={}; Object.defineProperty(o,'x',{value:1}); try{o.x=2}catch(e){e.name}",
        "var o={a:1}; Object.defineProperty(o,'a',{writable:false}); o.a=5; o.a",
        "var o={a:1}; Object.defineProperty(o,'a',{configurable:false}); try{Object.defineProperty(o,'a',{get:function(){return 2}})}catch(e){e.name}",
    ];
    for (i, c) in cases.iter().enumerate() {
        same(&cs, &rs, &format!("getter/setter case {}", i), c);
    }
}

#[test]
fn arguments_and_function_semantics() {
    let (cs, rs) = pair();
    let cases = [
        "function f(a,b){arguments[0]=9; return a}; f(1,2)",
        "function f(a,b){a=9; return arguments[0]}; f(1,2)",
        "function f(a){return arguments.length}; f()+','+f(1)+','+f(1,2)",
        "function f(){return Array.prototype.slice.call(arguments).join(',')}; f(1,2,3)",
        "function f(){return typeof arguments}; f()",
        "function f(){return Object.prototype.toString.call(arguments)}; f()",
        "function f(){return arguments.callee.length}; f.length+','+f()",
        "function f(){var s=''; for(var k in arguments) s+=k; return s}; f(1,2,3)",
        "function f(){return JSON.stringify(arguments)}; f(1,2)",
        "function f(){arguments.length=1; return arguments.length}; f(1,2,3)",
        "function f(){return delete arguments[0]}; f(1)",
        "function f(a){return (function(){return a})()}; f(7)",
        "function f(){return this}; String(f.call(1))+','+typeof f.call(1)",
        "function f(){return this}; typeof f.call('s')",
        "function f(){return this}; typeof f.call(null)",
        "function f(){'use strict'; return this}; String(f.call(1))",
        "function f(a,b){return a+b}; f.apply(null,[1,2])+','+f.call(null,1,2)",
        "try{ (function(){}).apply(null, 1) }catch(e){ e.name }",
        "try{ (function(){}).call() }catch(e){ e.name }",
        "(function(){return typeof arguments.callee})()",
        "var f=function g(){return typeof g}; f()+','+typeof g",
        "function f(){}; f.prototype.constructor===f",
        "function f(){}; Object.keys(f).length+','+Object.getOwnPropertyNames(f).sort().join(',')",
        "function f(a,b){}; Object.getOwnPropertyNames(f).sort().join(',')",
        "(function(){}).constructor===Function",
        "function f(){return new.target}; try{f()}catch(e){e.name}",
        "function C(){}; var o=new C(); Object.getPrototypeOf(o)===C.prototype",
        "function C(){return 5}; typeof new C()",
        "function C(){return {}}; typeof new C()",
        "function C(){this.x=1; return 5}; (new C()).x",
        "var bound=(function(a,b){return [this&&this.v,a,b].join('|')}).bind({v:1},2); bound(3)+';'+bound.length+';'+bound.name",
        "var B=(function(){this.z=1}).bind(null); var o=new B(); o.z",
    ];
    for (i, c) in cases.iter().enumerate() {
        same(&cs, &rs, &format!("arguments/function case {}", i), c);
    }
}

#[test]
fn gc_interleaved_with_work() {
    let cs = Session::new(Side::C, 0);
    let rs = Session::new(Side::Rust, 0);
    let steps = [
        "var keep=[]; for(var i=0;i<300;i++) keep.push({i:i, s:'str'+i}); keep.length",
        "var tmp=[]; for(var i=0;i<300;i++) tmp.push([i,i+1]); tmp.length",
        "tmp=null; 1",
        "var o={}; for(var i=0;i<300;i++) o['k'+i]={v:i}; Object.keys(o).length",
        "for(var i=0;i<150;i+=2) delete o['k'+i]; Object.keys(o).length",
        "var f=[]; for(var i=0;i<100;i++) f.push(function(){return i}); f.length",
        "var re=[]; for(var i=0;i<100;i++) re.push(new RegExp('a'+i)); re.length",
        "var d=[]; for(var i=0;i<100;i++) d.push(new Date(i*1e9)); d.length",
        "keep[0].i+','+Object.keys(o).length+','+f[0]()+','+String(re[0])+','+d[0].getTime()",
    ];
    for (i, s) in steps.iter().enumerate() {
        let a = run_script(&cs, s);
        let b = run_script(&rs, s);
        assert_eq!(a, b, "gc interleave step {}", i);
        unsafe { (cs.vm.gc)(cs.j, 0) };
        unsafe { (rs.vm.gc)(rs.j, 0) };
        let a = run_script(&cs, "typeof keep+','+typeof o+','+typeof f+','+typeof re+','+typeof d");
        let b = run_script(&rs, "typeof keep+','+typeof o+','+typeof f+','+typeof re+','+typeof d");
        assert_eq!(a, b, "gc interleave typeof after step {}", i);
    }
    // gc with report=1 goes through js_report
    cs.clear_logs();
    rs.clear_logs();
    unsafe { (cs.vm.gc)(cs.j, 1) };
    unsafe { (rs.vm.gc)(rs.j, 1) };
    let (a, b) = (cs.reports(), rs.reports());
    assert_eq!(a.len(), b.len(), "gc report count differs");
    // The exact counts must match too: they reflect the object graph.
    assert_eq!(a, b, "gc report text differs");
}
