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

fn parse_(s: &Vec<char>, mut i: usize, lvl: usize) -> Result<(usize, Parsed), ParseError> {
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
            let (i_, subtree) = parse_(s, i + 1, lvl + 1)?;
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

fn parse(s: &str) -> Result<Vec<Parsed>, ParseError> {
    let s_ = format!("{{{}}}", s);
    let (_, tree) = parse_(&s_.chars().collect(), 0, 0)?;

    if let Parsed::Tree(mut inner1) = tree {
        if let Parsed::Tree(inner2) = inner1.remove(1) {
            return Ok(inner2);
        }
    }
    panic!("unreachable")
}

fn to_regex_(tree: &[Parsed], is_group: bool, default: &str, result: &mut Vec<String>) {
    if is_group {
        let first = match tree[0] {Parsed::Str(ref s) => s, _ => {panic!("unreachable")} };

        let (group, rx) = if first.len() == 0 {
            ("", default) // TODO escape or verify default first?
        } else if let Some(i) = first.find(':') {
            (&first[..i], &first[i + 1..])
        } else {
            let rx = if first.chars().all(|c| c.is_ascii_digit()) {
                r"\d*"
            } else {
                default
            };
            (first.as_str(), rx)
        };

        result.push(format!("(?P<P_{}>{}", group, String::from(rx)));
        to_regex_(&tree[1..], false, default, result);
        result.push(String::from(")"));
    } else {
        for x in tree {
            match x {
                Parsed::Str(s) => result.push(s.to_string()),
                Parsed::Tree(ref subtree) => to_regex_(subtree, true, default, result),
            }
        }
    }
}

fn to_regex(tree: &[Parsed], default: &str) -> String {
    let mut result: Vec<String> = Vec::new();
    to_regex_(tree, false, default, &mut result);
    result.join("")
}

//fn to_subst(tree: &Parsed) -> Vec<String> {
//    let mut result: Vec<String> = Vec::new();
//    if let &Parsed::Tree(ref tree1) = tree {
//
//    }
//}

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

fn pat_subst(res: &Vec<Regex>, subs: &[Vec<&str>], x: &str) -> Option<String> {
    for (re, sub) in res.iter().by_ref().zip(subs.iter()) {
        if let Some(captures) = re.captures(x) {
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

            return Some(assemble(&sub[..], &dict))
        }
    }
    None
}


// APP

const DEFAULT: &str = r"[^/&?=]*";
const USAGE: &str =
"Usage: patsub [OPTIONS] [--] [PATTERN SUBSTITUTION]...

OPTIONS:
    -h           Show help.
    -v           Show version.
    -d           Set DEFAULT regex. Default = '\\w*'.
    -p           Show compiled regex patterns and quit.
    -b           buffered output.
    --version    Show version.

PATTERN RULES:
    {pat:regex}              define capture group named 'pat' matching 'regex'
    {pat:re{{nested:.*}x}}   define nested group named 'nested'
    {}                       named group equivalent to {a:\\[^/&?=]*}
    {1}                      numeric group matches numbers only = {1:\\d*}

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
        .arg(Arg::with_name("DEFAULT").short("d").value_name("DEFAULT").takes_value(true))
        .arg(Arg::with_name("PRINT").short("p"))
        .arg(Arg::with_name("BUFFERED").short("b"))
        .get_matches();

    let default = arg_matches.value_of("DEFAULT").unwrap_or(DEFAULT);

    // Parse command line patsubs
    let mut patsubs_: Vec<&str> = match arg_matches.values_of("PATSUB") {
        Some(values) => values.collect(),
        _ => Vec::new(),
    };
    if patsubs_.len() % 2 == 1 {
        patsubs_.push("{@}");
    }
    let patsubs: Vec<Vec<Parsed>> = patsubs_.iter().map(|x| {
        match parse(x) {
            Ok(parsed) => parsed,
            Err(e) => {
                println!("Couldn't parse'{}': {}", x, e);
                std::process::exit(1);
            },
        }
    }).collect();

    let res: Vec<Regex> = patsubs.iter().step_by(2).map(|tree| {
        match Regex::new(&to_regex(tree, default)) {
            Ok(x) => x,
            Err(e) => {
                println!("Couldn't parse regex: {}", e);
                std::process::exit(1);
            },
        }
    }).collect();

    // Compile substitutions
    let subs: Vec<Vec<&str>> = patsubs.iter().skip(1).step_by(2).map(|tree| {
        tree.iter().map(|x| {
            match x {
                Parsed::Str(s) => s.as_str(),
                Parsed::Tree(t) => {
                    if let Parsed::Str(s1) = &t[0] {
                        s1.as_str()
                    } else {
                        panic!("unreachable")
                    }
                }
            }
        }).collect()
    }).collect();


    // Print and exit
    if arg_matches.is_present("PRINT") {
        for (re, sub_) in res.iter().by_ref().zip(subs.iter()) {
            let mut sub: Vec<char> = Vec::new();
            for (i, &s) in sub_.iter().enumerate() {
                if i % 2 == 0 {
                    sub.extend(s.chars())
                } else {
                    sub.push('{');
                    sub.extend(s.chars());
                    sub.push('}');
                }
            }
            let sub_str: String = sub.iter().collect();
            let re_str = re.as_str();
            println!("{} -> {}", &re_str[1..re_str.len()], sub_str);
        }
        std::process::exit(0);
    }


    // Main loop
    for line in io::stdin().lock().lines() {
        match pat_subst(&res, &subs, &line.unwrap()) {
            Some(y) => {
                let _ = io::stdout().write_all(y.as_bytes());
                let _ = io::stdout().write_all(b"\n");
                if !arg_matches.is_present("BUFFERED") { // TODO is this really necessary (for performance)??
                    let _ = io::stdout().flush();
                }
            },
            None => {},
        }
    }
}


// TESTS

#[test]
fn test_parse() {
    let pattern = r"{b:batch_{1}.txt}";
    let compiled_regex = to_regex(&parse(pattern).unwrap(), DEFAULT);
    let expected_regex = r"(?P<P_b>batch_(?P<P_1>\d*).txt)";
    assert_eq!(compiled_regex, expected_regex);
}
