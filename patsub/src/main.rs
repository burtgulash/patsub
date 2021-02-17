use std::io::{self,BufRead,Write};
use std::collections::HashMap;
use std::fmt;

extern crate regex;
use regex::Regex;

extern crate clap;
use clap::{App,Arg};


#[derive(Debug)]
pub struct ParseError {
    msg: String
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}


#[derive(Debug)]
enum Parsed {
    Str(String),
    Tree(Vec<Parsed>),
}

fn parse(s: &Vec<char>, mut i: usize, lvl: usize) -> Result<(usize, Parsed), ParseError> {
    let mut buf: Vec<char> = Vec::new();
    let mut tree: Vec<Parsed> = Vec::new();
    let mut escape = false;

    while i < s.len() {
        let c = s[i];

        if escape {
            if i == s.len() - 1 {
                buf.push('\\');
                escape = false;
            } else {
                if "{}".contains(c) {
                    buf.push(c);
                } else if c == '\\' {
                    // Rust regex eats one '\'. Supply two to compensate
                    buf.push(c);
                    buf.push(c);
                } else {
                    buf.push('\\');
                    buf.push(c);
                }
                i += 1;
                escape = false;
                continue
            }
        }

        if c == '\\' {
            i += 1;
            escape = true;
            continue
        }

        if c == '{' {
            tree.push(Parsed::Str(buf.iter().collect()));
            let (i_, subtree) = parse(s, i + 1, lvl + 1)?;
            i = i_;
            tree.push(subtree);
            buf.clear();
        } else if c == '}' {
            tree.push(Parsed::Str(buf.iter().collect()));
            if lvl == 1 && i < s.len() - 1 {
                return Err(ParseError{ msg: format!("Parse error: closing }}\n\t{}", s[1..s.len() - 1].iter().collect::<String>()) });
            }
            return Ok((i, Parsed::Tree(tree)))
        } else {
            buf.push(c);
        }

        i += 1;
    }

    if lvl > 0 {
        return Err(ParseError{ msg: format!("Parse error: unclosed {{\n\t{}", s[1..s.len() - 1].iter().collect::<String>()) });
    }

    Ok((i, Parsed::Tree(tree)))
}

fn to_regex_(tree: &[Parsed], is_group: bool, delimiters: &str, result: &mut Vec<String>) {
    if is_group {
        let first = match tree[0] {Parsed::Str(ref s) => s, _ => {panic!("unreachable")} };

        let (group, rx) = if first.len() == 0 {
            ("", format!("[^{}]*", delimiters)) // TODO escape or verify delimiters first?
        } else if let Some(i) = first.find(':') {
            (&first[..i], String::from(&first[i + 1..]))
        } else {
            let rx = if first.chars().all(|c| c.is_ascii_lowercase()) {
                format!("[^{}]*", delimiters)
            } else if first.chars().all(|c| c.is_ascii_alphabetic()) {
                String::from(r"\w*")
            } else if first.chars().all(|c| c.is_ascii_digit()) {
                String::from(r"\d*")
            } else {
                String::from(r"[^/]*")
            };
            (first.as_str(), rx)
        };

        result.push(format!("(?P<P_{}>{}", group, rx));
        to_regex_(&tree[1..], false, delimiters, result);
        result.push(String::from(")"));
    } else {
        for x in tree {
            match x {
                Parsed::Str(s) => result.push(s.to_string()),
                Parsed::Tree(ref subtree) => to_regex_(subtree, true, delimiters, result),
            }
        }
    }
}

fn to_regex(tree: &Parsed, delimiters: &str) -> String {
    let mut result: Vec<String> = Vec::new();
    if let &Parsed::Tree(ref tree1) = tree {
        if let Parsed::Tree(ref tree2) = tree1[1] {
            to_regex_(tree2, false, delimiters, &mut result);
        }
    }
    result.join("")
}

fn assemble(s: &[&str], dict: &HashMap<&str, &str>) -> String {
    let mut res: Vec<char> = s[0].chars().collect();
    for i in (1..s.len()).step_by(2) {
        let key = format!("P_{}", s[i]);
        if let Some(m) = dict.get(key.as_str()) {
            res.extend(m.chars());
        }
        res.extend(s[i + 1].chars());
    }
    res.iter().collect()
}

const DELIMITERS: &str = "/&";
const USAGE: &str =
"Usage: patsub [OPTIONS] [--] [PATTERN SUBSTITUTION]...

OPTIONS:
    -h           Show help.
    -v           Show version.
    -d           Set DELIMITERS. Default = '/&'.
    -p           Show compiled regex patterns and quit.
    --version    Show version.

PATTERN RULES:
    {pat:regex}              define capture group named 'pat' matching 'regex'
    {pat:re{{nested:.*}x}}   define nested group named 'nested'
    {a}                      lowercase group = {a:[^DELIMITERS]*}
    {A}                      uppercase group = {a:\\w*}
    {1}                      numeric group   = {a:\\d*}

SPECIAL SUBSTITUTIONS:
    {%}                      matched text
    {@}                      complete input text
    {^}                      text before match
    {$}                      text after match

EXAMPLES:
    find /tmp | patsub /tmp/{file} {file}
    find /tmp | patsub '/tmp/{path:.*}/{dir}/{file}$' {dir}/{file}
    find /tmp | patsub '{file}$' '{^} -> {%} from {@}'";


fn main() {
    let arg_matches = App::new("patsub").version("0.1")
        .help(USAGE)
        .arg(Arg::with_name("PATSUB").multiple(true))
        .arg(Arg::with_name("DELIMITERS").short("d").value_name("DELIMITERS").takes_value(true))
        .arg(Arg::with_name("PRINT").short("p"))
        .get_matches();

    let patsubs: Vec<&str> = match arg_matches.values_of("PATSUB") {
        Some(values) => values.collect(),
        _ => Vec::new(),
    };
    let delimiters = arg_matches.value_of("DELIMITERS").unwrap_or(DELIMITERS);

    // Parse command line patsubs
    let mut pats_: Vec<&str> = Vec::new();
    let mut subs_: Vec<&str> = Vec::new();

    for i in (0..patsubs.len()).step_by(2) {
        pats_.push(patsubs[i].as_ref());
        if i + 1 == patsubs.len() {
            subs_.push(patsubs[i].as_ref());
        } else {
            subs_.push(patsubs[i + 1].as_ref());
        }
    }

    // Parse patterns
    let pats: Vec<String> = pats_.iter().map(|pat_| {
        let pat = format!("{{{}}}", pat_);
        match parse(&pat.chars().collect(), 0, 0) {
            Err(e) => {
                println!("Couldn't parse regex pattern '{}': {}", pat_, e);
                std::process::exit(1);
            },
            Ok((_, tree)) => format!("({})", to_regex(&tree, delimiters))
        }
    }).collect();

    let res: Vec<Regex> = pats.iter().map(|pat| {
        match Regex::new(&pat) {
            Result::Ok(x) => x,
            Result::Err(e) => {
                println!("Couldn't parse regex pattern: {}", e);
                std::process::exit(1);
            },
        }
    }).collect();

    // Compile substitutions
    let subs: Vec<Vec<&str>> = subs_.iter().map(|s| s.split(&['{', '}'][..]).collect()).collect();

    // Print and exit
    if arg_matches.is_present("PRINT") {
        for (re, sub_) in res.iter().by_ref().zip(subs_.iter()) {
            let re_str = re.as_str();
            println!("{} -> {}", &re_str[1..re_str.len()], sub_);
        }
        std::process::exit(0);
    }


    for line in io::stdin().lock().lines() {
        // safe to unwrap. Docs do it
        let x = line.unwrap();

        for (re, sub) in res.iter().by_ref().zip(subs.iter()) {
            if let Some(captures) = re.captures(x.as_str()) {

                // There is always at least one capture, because pattern was wrapped in ()
                // unwrap is safe
                let main_capture = captures.iter().next().unwrap().unwrap();

                let mut dict: HashMap<&str, &str> =
                    re
                    .capture_names()
                    .flatten()
                    .filter_map(|name| Some((name, captures.name(name)?.as_str())))
                    .collect();

                dict.insert("P_@", &x);
                dict.insert("P_%", &x[main_capture.start()..main_capture.end()]);
                dict.insert("P_^", &x[..main_capture.start()]);
                dict.insert("P_$", &x[main_capture.end()..]);

                let y = assemble(&sub[..], &dict);
                let _ = io::stdout().write_all(y.as_bytes());
                let _ = io::stdout().write_all(b"\n");
                let _ = io::stdout().flush();
            }
        }
    }
}
