# Scarlet

Scarlet is a dynamically typed, object-oriented, garbage-collected programming language written in Rust. It compiles source code to bytecode in a single pass and executes it on a stack-based virtual machine, with the entire runtime weighing in at just ~500 KB. Scarlet runs natively as a CLI tool and also compiles to WebAssembly for embedded use in browsers and Node.js environments.

---

## Features

- **Dynamic typing**: variables hold any value type at runtime
- **Object-oriented**: classes, single inheritance, `this`, `super`, and constructors via `init`
- **Closures**: first-class functions that capture variables from enclosing scopes via upvalues
- **Garbage collection**: tri-color mark-and-sweep GC
- **String interning**: identical strings share a single allocation, enabling O(1) equality checks
- **Bitwise operators**: `&`, `|`, `^`, `!` (bitwise NOT), `<<`, `>>`
- **REPL**: interactive read-eval-print loop with persistent state across lines
- **Debug/disassembly mode**: prints annotated bytecode before execution
- **Native functions**: built-in functions for I/O, type introspection, coercion, and timing
- **WebAssembly target**: compile to `.wasm` for web, bundler, and Node.js targets via `wasm-pack`

---

## Language Specification

### Types

| Type       | Examples                   |
| ---------- | -------------------------- |
| `number`   | `42`, `3.14`, `-7`         |
| `boolean`  | `true`, `false`            |
| `nil`      | `nil`                      |
| `string`   | `"hello"`, `"world"`       |
| `function` | defined with `fun`         |
| `class`    | defined with `class`       |
| `instance` | created by calling a class |

---

### Variables

Variables are declared with `let`. Uninitialized variables default to `nil`.

```scarlet
let x = 10;
let name = "scarlet";
let nothing;        // nil
```

---

### Operators

**Arithmetic**

```scarlet
1 + 2       // 3
10 - 4      // 6
3 * 5       // 15
10 / 4      // 2.5
10 % 3      // 1
```

**Comparison**

```scarlet
1 == 1      // true
1 != 2      // true
3 > 2       // true
3 >= 3      // true
2 < 5       // true
2 <= 2      // true
```

**Logical**

```scarlet
true && false   // false
true || false   // true
!true           // false
```

**Bitwise** (integer operands only, no fractional part)

```scarlet
6 & 3       // 2   (AND)
6 | 3       // 7   (OR)
6 ^ 3       // 5   (XOR)
!5          // bitwise NOT, same as Rust
2 << 3      // 16  (left shift)
16 >> 2     // 4   (right shift)
```

**String concatenation**

```scarlet
"hello" + " " + "world"    // "hello world"
```

---

### Control Flow

**if / else**

```scarlet
if (x > 0) {
    println("positive");
} else {
    println("non-positive");
}
```

**while**

```scarlet
let i = 0;
while (i < 5) {
    println(i);
    i = i + 1;
}
```

**for**

```scarlet
for (let i = 0; i < 5; i = i + 1) {
    println(i);
}
```

---

### Functions

Functions are first-class values declared with `fun`. They support recursion and closures.

```scarlet
fun add(a, b) {
    return a + b;
}

println(add(3, 4));   // 7
```

**Closures**

```scarlet
fun make_counter() {
    let count = 0;
    fun increment() {
        count = count + 1;
        return count;
    }
    return increment;
}

let counter = make_counter();
println(counter());   // 1
println(counter());   // 2
```

---

### Classes

Classes are declared with `class`. The special `init` method is the constructor. Use `this` to reference the current instance.

```scarlet
class Animal {
    init(name) {
        this.name = name;
    }

    speak() {
        println(this.name + " makes a sound.");
    }
}

let a = Animal("Dog");
a.speak();    // Dog makes a sound.
```

**Inheritance**

Use the `inherits` keyword to extend a class. Call superclass methods with `super`.

```scarlet
class Dog inherits Animal {
    init(name) {
        super.init(name);
    }

    speak() {
        println(this.name + " barks.");
    }
}

let d = Dog("Rex");
d.speak();    // Rex barks.
```

---

### Native Functions

These functions are built in and available globally.

| Function       | Description                                         | Platform    |
| -------------- | --------------------------------------------------- | ----------- |
| `print(...)`   | Print values without a trailing newline             | All         |
| `println(...)` | Print values followed by a newline                  | All         |
| `len(s)`       | Return the length of a string                       | All         |
| `type(v)`      | Return the type of a value as a string              | All         |
| `to_string(v)` | Convert a value to its string representation        | All         |
| `to_number(v)` | Parse a string or coerce a boolean to a number      | All         |
| `clock()`      | Return the current Unix timestamp in milliseconds   | Native only |
| `sleep(ms)`    | Sleep for the given number of milliseconds          | Native only |
| `read(prompt)` | Print a prompt, read a line of stdin, and return it | Native only |

```scarlet
let t = type(42);           // "number"
let s = to_string(3.14);    // "3.14"
let n = to_number("99");    // 99
println(len("hello"));      // 5
```

---

### Comments

Only single-line comments are supported, using `//`.

```scarlet
// This is a comment
let x = 1;   // inline comment
```

---

## Architecture

Scarlet is organized as a classic pipeline with a separate WebAssembly entry point.

Source text is fed into the **Scanner**, a lexer that produces a stream of tokens. Those tokens flow into the **Compiler**, a hand-crafted single-pass Pratt parser that consumes the token stream and directly emits bytecode, with no AST built. At runtime, the **Virtual Machine** executes that bytecode on a stack-based interpreter, handling function calls, closures, and triggering garbage collection. All heap-allocated objects (strings, functions, closures, class instances) live in the **Heap**, which also owns the string intern table and the globals table.

The garbage collector uses a mark-and-sweep strategy, tracing live objects from the VM stack, call frames, open upvalues, and globals, then sweeping anything unreachable.

The entire compiler and virtual machine weigh in at roughly **~500 KB** as a release binary.

---

## Building

### Native binary

```bash
cargo build --bin scarlet --release
```

### WebAssembly: web target

```bash
wasm-pack build --target web
```

### WebAssembly: bundler target (webpack, vite, etc.)

```bash
wasm-pack build --target bundler
```

### WebAssembly: Node.js target

```bash
wasm-pack build --target nodejs
```

All `wasm-pack` outputs are written to `pkg/`.

---

## Usage

### Run a script

The `.scar` file extension is the convention for Scarlet script files.

```bash
scarlet --run path/to/script.scar
```

### Start the REPL

```bash
scarlet --repl
# or just
scarlet
```

### Debug mode (disassemble bytecode)

```bash
scarlet --run script.scar --debug
scarlet --repl --debug
```

---

## License

MIT

## Note

Scarlet is built for educational purposes, heavily inspired by the clox implementation of the Lox language from the excellent _Crafting Interpreters_ book by Bob Nystrom.
