use ascii_table::{Align, AsciiTable};
use rival::{Discretization, Execution, Expr, Ival, MachineBuilder, RivalError};
use rug::{Assign, Float, Rational};
use std::env;
use std::fmt::Display;

#[derive(Clone)]
struct Fp64Discretization;

impl Discretization for Fp64Discretization {
    fn target(&self) -> u32 {
        53
    }

    fn convert(&self, _idx: usize, v: &Float) -> Float {
        v.clone()
    }

    fn distance(&self, _idx: usize, lo: &Float, hi: &Float) -> usize {
        let x = lo.to_f64();
        let y = hi.to_f64();
        // Handle things like signed zeros (so that -0.0 == 0.0).
        if x == y {
            return 0;
        }

        let to_ordinal = |v: f64| -> i64 {
            let bits = v.to_bits() as i64;
            if bits < 0 { !bits } else { bits }
        };

        let ox = to_ordinal(x);
        let oy = to_ordinal(y);
        oy.wrapping_sub(ox).unsigned_abs() as usize
    }
}

enum SExpr {
    Atom(String),
    List(Vec<SExpr>),
}

fn parse_sexpr(s: &str) -> Result<SExpr, String> {
    let mut chars = s.trim().chars().peekable();
    let res = parse_sexpr_inner(&mut chars)?;
    if chars.peek().is_some() {
        return Err("Trailing characters after S-expression".to_string());
    }
    Ok(res)
}

fn parse_sexpr_inner<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
) -> Result<SExpr, String> {
    // Skip whitespace.
    while chars.peek().map_or(false, |c| c.is_whitespace()) {
        chars.next();
    }

    match chars.peek() {
        Some('(') => {
            chars.next(); // consume '('.
            let mut list = Vec::new();
            loop {
                // Skip whitespace inside list.
                while chars.peek().map_or(false, |c| c.is_whitespace()) {
                    chars.next();
                }

                match chars.peek() {
                    Some(')') => {
                        chars.next(); // consume ')'.
                        return Ok(SExpr::List(list));
                    }
                    Some(_) => {
                        list.push(parse_sexpr_inner(chars)?);
                    }
                    None => return Err("Unclosed parenthesis".to_string()),
                }
            }
        }
        Some(')') => Err("Unexpected closing parenthesis".to_string()),
        Some(_) => {
            let mut atom = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() || c == '(' || c == ')' {
                    break;
                }
                atom.push(c);
                chars.next();
            }
            if atom.is_empty() {
                Err("Unexpected end of input".to_string())
            } else {
                Ok(SExpr::Atom(atom))
            }
        }
        None => Err("Unexpected end of input".to_string()),
    }
}

fn parse_vars(s: &str) -> Result<Vec<String>, String> {
    let sexpr = parse_sexpr(s)?;
    match sexpr {
        SExpr::List(items) => items
            .into_iter()
            .map(|item| match item {
                SExpr::Atom(s) => Ok(s),
                _ => Err("Variable list must contain atoms".to_string()),
            })
            .collect(),
        _ => Err("Variables must be a list".to_string()),
    }
}

fn parse_values(s: &str) -> Result<Vec<String>, String> {
    let sexpr = parse_sexpr(s)?;
    match sexpr {
        SExpr::List(items) => items
            .into_iter()
            .map(|item| match item {
                SExpr::Atom(s) => Ok(s),
                _ => Err("Value list must contain atoms".to_string()),
            })
            .collect(),
        _ => Err("Values must be a list".to_string()),
    }
}

fn sexpr_to_expr(sexpr: SExpr, vars: &[String]) -> Result<Expr, String> {
    match sexpr {
        SExpr::Atom(s) => {
            if vars.contains(&s) {
                return Ok(Expr::Var(s));
            }
            if let Ok(rat) = s.parse::<Rational>() {
                return Ok(Expr::Rational(rat));
            }
            if let Ok(f) = Float::parse(&s) {
                return Ok(Expr::Literal(Float::with_val(1024, f)));
            }
            match s.to_uppercase().as_str() {
                "PI" => Ok(Expr::Pi),
                "E" => Ok(Expr::E),
                _ => Err(format!("Unknown atom: {}", s)),
            }
        }
        SExpr::List(items) => {
            if items.is_empty() {
                return Err("Empty list".to_string());
            }
            let op = match &items[0] {
                SExpr::Atom(s) => s.clone(),
                _ => return Err("Operator must be an atom".to_string()),
            };
            let args: Result<Vec<_>, _> = items
                .into_iter()
                .skip(1)
                .map(|item| sexpr_to_expr(item, vars))
                .collect();
            let args = args?;
            match (op.as_str(), args.len()) {
                ("+", 2) => Ok(Expr::Add(
                    Box::new(args[0].clone()),
                    Box::new(args[1].clone()),
                )),
                ("-", 1) => Ok(Expr::Neg(Box::new(args[0].clone()))),
                ("-", 2) => Ok(Expr::Sub(
                    Box::new(args[0].clone()),
                    Box::new(args[1].clone()),
                )),
                ("*", 2) => Ok(Expr::Mul(
                    Box::new(args[0].clone()),
                    Box::new(args[1].clone()),
                )),
                ("/", 2) => Ok(Expr::Div(
                    Box::new(args[0].clone()),
                    Box::new(args[1].clone()),
                )),
                ("pow", 2) => Ok(Expr::Pow(
                    Box::new(args[0].clone()),
                    Box::new(args[1].clone()),
                )),
                ("sqrt", 1) => Ok(Expr::Sqrt(Box::new(args[0].clone()))),
                ("cbrt", 1) => Ok(Expr::Cbrt(Box::new(args[0].clone()))),
                ("exp", 1) => Ok(Expr::Exp(Box::new(args[0].clone()))),
                ("exp2", 1) => Ok(Expr::Exp2(Box::new(args[0].clone()))),
                ("expm1", 1) => Ok(Expr::Expm1(Box::new(args[0].clone()))),
                ("log", 1) => Ok(Expr::Log(Box::new(args[0].clone()))),
                ("log2", 1) => Ok(Expr::Log2(Box::new(args[0].clone()))),
                ("log10", 1) => Ok(Expr::Log10(Box::new(args[0].clone()))),
                ("log1p", 1) => Ok(Expr::Log1p(Box::new(args[0].clone()))),
                ("sin", 1) => Ok(Expr::Sin(Box::new(args[0].clone()))),
                ("cos", 1) => Ok(Expr::Cos(Box::new(args[0].clone()))),
                ("tan", 1) => Ok(Expr::Tan(Box::new(args[0].clone()))),
                ("asin", 1) => Ok(Expr::Asin(Box::new(args[0].clone()))),
                ("acos", 1) => Ok(Expr::Acos(Box::new(args[0].clone()))),
                ("atan", 1) => Ok(Expr::Atan(Box::new(args[0].clone()))),
                ("sinh", 1) => Ok(Expr::Sinh(Box::new(args[0].clone()))),
                ("cosh", 1) => Ok(Expr::Cosh(Box::new(args[0].clone()))),
                ("tanh", 1) => Ok(Expr::Tanh(Box::new(args[0].clone()))),
                ("asinh", 1) => Ok(Expr::Asinh(Box::new(args[0].clone()))),
                ("acosh", 1) => Ok(Expr::Acosh(Box::new(args[0].clone()))),
                ("atanh", 1) => Ok(Expr::Atanh(Box::new(args[0].clone()))),
                ("fabs", 1) => Ok(Expr::Fabs(Box::new(args[0].clone()))),
                ("neg", 1) => Ok(Expr::Neg(Box::new(args[0].clone()))),
                ("hypot", 2) => Ok(Expr::Hypot(
                    Box::new(args[0].clone()),
                    Box::new(args[1].clone()),
                )),
                ("atan2", 2) => Ok(Expr::Atan2(
                    Box::new(args[0].clone()),
                    Box::new(args[1].clone()),
                )),
                ("fmin", 2) => Ok(Expr::Fmin(
                    Box::new(args[0].clone()),
                    Box::new(args[1].clone()),
                )),
                ("fmax", 2) => Ok(Expr::Fmax(
                    Box::new(args[0].clone()),
                    Box::new(args[1].clone()),
                )),
                ("fmod", 2) => Ok(Expr::Fmod(
                    Box::new(args[0].clone()),
                    Box::new(args[1].clone()),
                )),
                ("remainder", 2) => Ok(Expr::Remainder(
                    Box::new(args[0].clone()),
                    Box::new(args[1].clone()),
                )),
                ("copysign", 2) => Ok(Expr::Copysign(
                    Box::new(args[0].clone()),
                    Box::new(args[1].clone()),
                )),
                ("fdim", 2) => Ok(Expr::Fdim(
                    Box::new(args[0].clone()),
                    Box::new(args[1].clone()),
                )),
                ("erf", 1) => Ok(Expr::Erf(Box::new(args[0].clone()))),
                ("erfc", 1) => Ok(Expr::Erfc(Box::new(args[0].clone()))),
                ("lgamma", 1) => Ok(Expr::Lgamma(Box::new(args[0].clone()))),
                ("tgamma", 1) => Ok(Expr::Tgamma(Box::new(args[0].clone()))),
                ("floor", 1) => Ok(Expr::Floor(Box::new(args[0].clone()))),
                ("ceil", 1) => Ok(Expr::Ceil(Box::new(args[0].clone()))),
                ("round", 1) => Ok(Expr::Round(Box::new(args[0].clone()))),
                ("trunc", 1) => Ok(Expr::Trunc(Box::new(args[0].clone()))),
                ("rint", 1) => Ok(Expr::Rint(Box::new(args[0].clone()))),
                ("logb", 1) => Ok(Expr::Logb(Box::new(args[0].clone()))),
                (op, _) => Err(format!("Unknown or invalid arity for operator: {}", op)),
            }
        }
    }
}

fn display_table(execs: &[Execution], num_iterations: usize) {
    let num_cols = 1 + num_iterations * 2;

    let get_exec = |iter: usize, id: i32| -> Option<&Execution> {
        execs.iter().find(|e| e.iteration == iter && e.number == id)
    };

    let mut unique_ids: Vec<i32> = execs
        .iter()
        .filter_map(|e| (e.number >= 0).then_some(e.number))
        .collect();
    unique_ids.sort_unstable();
    unique_ids.dedup();

    let mut header = vec!["Name".to_string()];
    for iter in 0..num_iterations {
        header.push(format!("{} Bits", iter));
        header.push(format!("{} Time", iter));
    }

    let mut data: Vec<Vec<String>> = Vec::new();

    let mut adjust_row = vec!["adjust".to_string()];
    for col in 1..num_cols {
        if col % 2 == 0 && col >= 2 {
            let iter = col / 2 - 1;
            let cell = get_exec(iter, -1)
                .map(|e| format!("{:.1} µs", e.time_ms * 1000.0))
                .unwrap_or_default();
            adjust_row.push(cell);
        } else {
            adjust_row.push(String::new());
        }
    }
    data.push(adjust_row);

    for &id in &unique_ids {
        let name = execs
            .iter()
            .find(|e| e.number == id)
            .map(|e| e.name.to_string())
            .unwrap_or_default();
        let mut row = vec![name];

        for col in 1..num_cols {
            let cell = if col % 2 == 1 {
                let iter = (col - 1) / 2;
                get_exec(iter, id)
                    .map(|e| e.precision.to_string())
                    .unwrap_or_default()
            } else {
                let iter = col / 2 - 1;
                get_exec(iter, id)
                    .map(|e| format!("{:.1} µs", e.time_ms * 1000.0))
                    .unwrap_or_default()
            };
            row.push(cell);
        }
        data.push(row);
    }

    let mut total_row = vec!["Total".to_string()];
    for col in 1..num_cols {
        if col % 2 == 0 && col >= 2 {
            let iter = col / 2 - 1;
            let total: f64 = execs
                .iter()
                .filter(|e| e.iteration == iter)
                .map(|e| e.time_ms)
                .sum();
            total_row.push(format!("{:.1} µs", total * 1000.0));
        } else {
            total_row.push(String::new());
        }
    }
    data.push(total_row);

    let mut table = AsciiTable::default();
    table.set_max_width(240);
    table.column(0).set_header("Name").set_align(Align::Left);
    for (i, name) in header.iter().enumerate().skip(1) {
        table.column(i).set_header(name).set_align(Align::Right);
    }
    let display_data: Vec<Vec<&dyn Display>> = data
        .iter()
        .map(|row| row.iter().map(|cell| cell as &dyn Display).collect())
        .collect();

    println!();
    table.print(display_data);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: {} <expr> <vars> <values>", args[0]);
        eprintln!(
            "Example: {} \"(* (+ 2 (pow x y)) 23/43)\" \"(x y)\" \"(1e-25 5.0)\"",
            args[0]
        );
        std::process::exit(1);
    }

    let expr_str = &args[1];
    let vars_str = &args[2];
    let values_str = &args[3];

    let vars = parse_vars(vars_str).unwrap_or_else(|e| {
        eprintln!("Error parsing variables: {}", e);
        std::process::exit(1);
    });

    let values = parse_values(values_str).unwrap_or_else(|e| {
        eprintln!("Error parsing values: {}", e);
        std::process::exit(1);
    });

    if vars.len() != values.len() {
        eprintln!("Number of variables and values must match");
        std::process::exit(1);
    }

    let sexpr = parse_sexpr(expr_str).unwrap_or_else(|e| {
        eprintln!("Error parsing expression: {}", e);
        std::process::exit(1);
    });

    let expr = sexpr_to_expr(sexpr, &vars).unwrap_or_else(|e| {
        eprintln!("Error converting expression: {}", e);
        std::process::exit(1);
    });

    let mut machine = MachineBuilder::new(Fp64Discretization)
        .enable_profiling(true)
        .max_precision(10000)
        .build(vec![expr], vars);

    let arg_prec = machine.argument_precision();
    let arg_ivals: Vec<Ival> = values
        .iter()
        .map(|s| {
            let mut ival = Ival::zero(arg_prec);
            if let Ok(rat) = s.parse::<Rational>() {
                let f = Float::with_val(arg_prec, &rat);
                ival.lo_mut().assign(&f);
                ival.hi_mut().assign(&f);
            } else if let Ok(f) = Float::parse(s) {
                let f = Float::with_val(arg_prec, f);
                ival.lo_mut().assign(&f);
                ival.hi_mut().assign(&f);
            } else if let Ok(v) = s.parse::<f64>() {
                ival.f64_assign(v);
            } else {
                panic!("Invalid value: {}", s);
            }
            ival
        })
        .collect();

    // Warm-up run just like the racket repl.
    let _ = machine.apply(&arg_ivals, None, 5);

    let start = std::time::Instant::now();
    let result = machine.apply(&arg_ivals, None, 10);
    let total_time = start.elapsed().as_secs_f64() * 1000.0;

    let execs: Vec<Execution> = machine.execution_records().to_vec();
    let num_iterations = execs.iter().map(|e| e.iteration).max().unwrap_or(0) + 1;
    let num_instructions = machine.instruction_count();

    println!(
        "Executed {} instructions for {} iterations:",
        num_instructions, num_iterations
    );
    display_table(&execs, num_iterations);

    match result {
        Ok(outputs) => {
            print!("\nFinal value:");
            for output in outputs {
                let lo = output.lo();
                let hi = output.hi();
                if lo == hi {
                    print!(" {}", lo.to_f64());
                } else {
                    print!(" [{}, {}]", lo.to_f64(), hi.to_f64());
                }
            }
            println!();
            println!("Total: {:.1} µs", total_time * 1000.0);
        }
        Err(RivalError::InvalidInput) => {
            println!("\nError: Invalid input");
        }
        Err(RivalError::Unsamplable) => {
            println!("\nError: Could not converge");
        }
    }
}
