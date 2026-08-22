use ascii_table::{Align, AsciiTable};
use rival::{
    Discretization, Execution, Expression, ExpressionBuilder, Ival, MachineBuilder, OutputPolicy,
    RivalError,
};
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

fn sexpr_to_expr(sexpr: SExpr, builder: &mut ExpressionBuilder) -> Result<Expression, String> {
    match sexpr {
        SExpr::Atom(s) => {
            if let Some(variable) = builder.variable(&s) {
                return Ok(variable);
            }
            if let Ok(rat) = s.parse::<Rational>() {
                return Ok(builder.rational(rat));
            }
            if let Ok(f) = Float::parse(&s) {
                return Ok(builder.literal(Float::with_val(1024, f)));
            }
            match s.to_uppercase().as_str() {
                "PI" => Ok(builder.pi()),
                "E" => Ok(builder.e()),
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
                .map(|item| sexpr_to_expr(item, builder))
                .collect();
            let args = args?;
            match (op.as_str(), args.len()) {
                ("+", 2) => Ok(builder.add(args[0], args[1])),
                ("-", 1) => Ok(builder.neg(args[0])),
                ("-", 2) => Ok(builder.sub(args[0], args[1])),
                ("*", 2) => Ok(builder.mul(args[0], args[1])),
                ("/", 2) => Ok(builder.div(args[0], args[1])),
                ("pow", 2) => Ok(builder.pow(args[0], args[1])),
                ("sqrt", 1) => Ok(builder.sqrt(args[0])),
                ("cbrt", 1) => Ok(builder.cbrt(args[0])),
                ("exp", 1) => Ok(builder.exp(args[0])),
                ("exp2", 1) => Ok(builder.exp2(args[0])),
                ("expm1", 1) => Ok(builder.expm1(args[0])),
                ("log", 1) => Ok(builder.log(args[0])),
                ("log2", 1) => Ok(builder.log2(args[0])),
                ("log10", 1) => Ok(builder.log10(args[0])),
                ("log1p", 1) => Ok(builder.log1p(args[0])),
                ("sin", 1) => Ok(builder.sin(args[0])),
                ("cos", 1) => Ok(builder.cos(args[0])),
                ("tan", 1) => Ok(builder.tan(args[0])),
                ("asin", 1) => Ok(builder.asin(args[0])),
                ("acos", 1) => Ok(builder.acos(args[0])),
                ("atan", 1) => Ok(builder.atan(args[0])),
                ("sinh", 1) => Ok(builder.sinh(args[0])),
                ("cosh", 1) => Ok(builder.cosh(args[0])),
                ("tanh", 1) => Ok(builder.tanh(args[0])),
                ("asinh", 1) => Ok(builder.asinh(args[0])),
                ("acosh", 1) => Ok(builder.acosh(args[0])),
                ("atanh", 1) => Ok(builder.atanh(args[0])),
                ("fabs", 1) => Ok(builder.fabs(args[0])),
                ("neg", 1) => Ok(builder.neg(args[0])),
                ("hypot", 2) => Ok(builder.hypot(args[0], args[1])),
                ("atan2", 2) => Ok(builder.atan2(args[0], args[1])),
                ("fmin", 2) => Ok(builder.fmin(args[0], args[1])),
                ("fmax", 2) => Ok(builder.fmax(args[0], args[1])),
                ("fmod", 2) => Ok(builder.fmod(args[0], args[1])),
                ("remainder", 2) => Ok(builder.remainder(args[0], args[1])),
                ("copysign", 2) => Ok(builder.copysign(args[0], args[1])),
                ("fdim", 2) => Ok(builder.fdim(args[0], args[1])),
                ("erf", 1) => Ok(builder.erf(args[0])),
                ("erfc", 1) => Ok(builder.erfc(args[0])),
                ("lgamma", 1) => Ok(builder.lgamma(args[0])),
                ("tgamma", 1) => Ok(builder.tgamma(args[0])),
                ("floor", 1) => Ok(builder.floor(args[0])),
                ("ceil", 1) => Ok(builder.ceil(args[0])),
                ("round", 1) => Ok(builder.round(args[0])),
                ("trunc", 1) => Ok(builder.trunc(args[0])),
                ("rint", 1) => Ok(builder.rint(args[0])),
                ("logb", 1) => Ok(builder.logb(args[0])),
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

    let mut expressions = ExpressionBuilder::new(vars.clone());
    let expr = sexpr_to_expr(sexpr, &mut expressions).unwrap_or_else(|e| {
        eprintln!("Error converting expression: {}", e);
        std::process::exit(1);
    });

    let mut machine = MachineBuilder::new(Fp64Discretization)
        .enable_profiling(true)
        .max_precision(10000)
        .build(&expressions, &[expr]);

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
    let _ = machine.apply(&arg_ivals, None, 5, OutputPolicy::AllowPartial);

    let start = std::time::Instant::now();
    let result = machine.apply(&arg_ivals, None, 10, OutputPolicy::AllowPartial);
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
