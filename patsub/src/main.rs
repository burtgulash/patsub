use std::env;
use std::io::{self,BufRead,Write};

extern crate regex;
use regex::Regex;

#[derive(Debug)]
enum Parsed {
    Str(String),
    Tree(Vec<Parsed>),
}

fn parse(s: &Vec<char>, mut i: usize, lvl: usize) -> (usize, Parsed) {
    let mut buf: Vec<char> = Vec::new();
    let mut tree: Vec<Parsed> = Vec::new();

    while i < s.len() {
        if s[i] == '{' {
            tree.push(Parsed::Str(buf.iter().collect()));
            let (i_, subtree) = parse(s, i + 1, lvl + 1);
            i = i_;
            tree.push(subtree);
            buf.clear();
        } else if s[i] == '}' {
            tree.push(Parsed::Str(buf.iter().collect()));
            if lvl == 0 && i < s.len() - 1 {
                //raise ValueError("Parse error: closing }")
            }
            return (i, Parsed::Tree(tree))
        } else {
            buf.push(s[i]);
        }

        i += 1;
    }

    (i, Parsed::Tree(tree))
}

fn to_regex_(tree: &[Parsed], is_group: bool, result: &mut Vec<String>) {
    if is_group {
        let name = match tree[0] {Parsed::Str(ref s) => s, _ => {panic!("unreachable")} };
        let rx = "[^/]*";
        result.push(format!("(?P<{}>{}", name, rx));

        to_regex_(&tree[1..], false, result);
        result.push(String::from(")"));
    } else {
        for x in tree {
            match x {
                Parsed::Str(s) => result.push(s.to_string()),
                Parsed::Tree(ref subtree) => to_regex_(subtree, true, result),
            }
        }
    }
}

fn to_regex(tree: &Parsed) -> String {
    let mut result: Vec<String> = Vec::new();
    if let &Parsed::Tree(ref tree1) = tree {
        if let Parsed::Tree(ref tree2) = tree1[1] {
            println!("yyy {:?}", tree2);
            to_regex_(tree2, false, &mut result);
        }
    }
    result.join("")
}

fn assemble(s: &[String]) -> String {
    let mut res: Vec<char> = s[0].chars().collect();
    for i in (1..s.len()).step_by(2) {
        res.push('<');
        res.extend(s[i].chars());
        res.push('>');
        res.extend(s[i + 1].chars());
    }
    res.iter().collect()
}

fn parse_sub(s: &str) -> Vec<String> {
    let mut res = Vec::new();
    for part in s.split(&['{', '}'][..]) {
        res.push(String::from(part));
    }
    res
}

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("argz {:?}", args);

//    let pat = format!("{{{}}}", &args[1]);
//    let (_, tree) = parse(&pat.chars().collect(), 0, 0);
//    let rx = to_regex(&tree);
//    println!("rx {}", rx);

    let mut pats_: Vec<&str> = Vec::new();
    let mut subs_: Vec<&str> = Vec::new();

    for i in (1..args.len()).step_by(2) {
        pats_.push(args[i].as_ref());
        if i + 1 == args.len() {
            subs_.push(args[i].as_ref());
        } else {
            subs_.push(args[i + 1].as_ref());
        }
    }
    println!("{:?}", pats_);
    println!("{:?}", subs_);

    let pats: Vec<Regex> = pats_.iter().map(|arg| {
        let pat = format!("{{{}}}", arg);
        let (_, tree) = parse(&pat.chars().collect(), 0, 0);
        let rx = to_regex(&tree);
        //println!("rx {}", rx);

        Regex::new(&rx).unwrap()
    }).collect();

    let subs: Vec<Vec<String>> = subs_.iter().map(|x| parse_sub(x)).collect();
    println!("DEBUG {:?}", subs);

    for line in io::stdin().lock().lines() {
        let x = line.unwrap();
        for (pat, sub) in pats.iter().by_ref().zip(subs.iter()) {
            //println!("PAT {:?}", pat);
            if let Some(m) = pat.find(x.as_str()) {
                let y = assemble(&sub[..]);
                let _ = io::stdout().write_all(y.as_bytes());
                let _ = io::stdout().write_all(b"\n");
                let _ = io::stdout().flush();
            }
        }
    }
}
