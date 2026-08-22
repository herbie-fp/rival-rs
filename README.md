# rival3
Rival is a library for evaluating real expressions to high precision. You can play around with the CLI:

```
$ cargo run --bin rival-cli --features cli --release -- "(- (sqrt (+ x 1)) 1)" "(x)" "(1e-30)"
```
Will give you:
```
Executed 4 instructions for 2 iterations:

┌────────┬────────┬────────┬────────┬────────┐
│ Name   │ 0 Bits │ 0 Time │ 1 Bits │ 1 Time │
├────────┼────────┼────────┼────────┼────────┤
│ adjust │        │        │        │ 0.5 µs │
│ Add    │     62 │ 0.3 µs │    633 │ 0.2 µs │
│ Sqrt   │     60 │ 0.8 µs │    632 │ 1.6 µs │
│ Sub    │     58 │ 0.2 µs │     58 │ 0.2 µs │
│ Total  │        │ 1.2 µs │        │ 2.6 µs │
└────────┴────────┴────────┴────────┴────────┘

Final value: [0.0000000000000000000000000000005, 0.0000000000000000000000000000005]
Total: 5.2 µs
```

