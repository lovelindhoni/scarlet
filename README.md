# Scarlet

Scarlet is an **AI-first**, dynamically typed, object-oriented, garbage-collected scripting language written in Rust. It brings LLM reasoning directly into the language as first-class primitives — no libraries, no API wrappers, just native keywords. Write scripts that think, classify, verify, and extract using natural language, right alongside your regular logic.

Scarlet compiles source code to bytecode in a single pass and executes it on a stack-based virtual machine. The entire runtime weighs ~500 KB. It runs natively as a CLI tool and compiles to WebAssembly for browser and Node.js environments.

---

## AI Primitives

Scarlet treats AI as a core part of the language, not an afterthought. Four built-in keywords let you call an LLM inline, anywhere an expression is valid. All primitives are powered by the Cerebras inference API and require `CEREBRAS_API_KEY` to be set in your environment.

```bash
export CEREBRAS_API_KEY=your_key_here
```

If the key is missing, Scarlet crashes immediately with a clear error. If the API call fails for any reason (network error, bad response), it also crashes with the error message.

---

### `generate` — text generation

Sends a prompt to the LLM and returns its response as a string.

```scarlet
let capital = generate "What is the capital of France?";
println(capital);   // Paris

let poem = generate "Write a two-line poem about the ocean";
println(poem);
```

---

### `verify` — boolean reasoning

Asks the LLM a yes/no question and returns `true` or `false`.

```scarlet
if (verify "Is water a renewable resource?") {
    println("yes");
} else {
    println("no");
}

let rich = verify "Is Adam Sandler a billionaire?";
println(rich);   // false
```

Use it in any boolean context: `if`, `while`, `&&`, `||`, variable assignment.

---

### `classify` — label classification

Classifies text into one of the provided labels. Returns the matching label as a string.

```scarlet
let review = "The battery dies after two hours and the screen scratches easily.";
let sentiment = classify review as "positive", "negative", "neutral";
println(sentiment);   // negative

let lang = classify "Bonjour le monde" as "English", "French", "Spanish", "German";
println(lang);   // French
```

---

### `extract` — information extraction

Extracts a specific piece of information from a source text. Returns the extracted value as a string.

```scarlet
let bio = "Ada Lovelace was born in London in 1815 and is credited as the first computer programmer.";
let birthplace = extract "birthplace" from bio;
println(birthplace);   // London

let year = extract "birth year" from bio;
println(year);   // 1815
```

---

### Combining AI primitives with language features

Because these are plain expressions, they compose naturally with everything else in the language.

```scarlet
// Use generate inside string concatenation
let lang = "Rust";
let summary = generate "Summarize " + lang + " in one sentence";

// classify driving a loop
let inputs = ...;
for (let i = 0; i < len(inputs); i = i + 1) {
    let category = classify inputs[i] as "spam", "not spam";
    println(category);
}

// verify guarding a branch
if (verify "Is the user's input a valid email address?") {
    // proceed
}
```

---

## Language Features

- **Dynamic typing** — variables hold any value type at runtime
- **Object-oriented** — classes, single inheritance, `this`, `super`, constructors via `init`
- **Closures** — first-class functions that capture outer variables via upvalues
- **Garbage collection** — tri-color mark-and-sweep GC
- **String interning** — identical strings share one allocation, O(1) equality
- **Bitwise operators** — `&`, `|`, `^`, `!` (bitwise NOT), `<<`, `>>`
- **REPL** — interactive session with persistent state across lines
- **Debug mode** — annotated bytecode disassembly before execution
- **Native functions** — I/O, type introspection, coercion, timing
- **WebAssembly target** — compile to `.wasm` for web and Node.js via `wasm-pack`

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
!5          // bitwise NOT
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

Use `inherits` to extend a class. Call superclass methods with `super`.

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

Scarlet is a classic pipeline with a separate WebAssembly entry point.

Source text goes into the **Scanner**, a lexer producing a token stream. Tokens flow into the **Compiler**, a single-pass Pratt parser that emits bytecode directly with no AST. The **Virtual Machine** executes that bytecode on a stack-based interpreter, handling calls, closures, and GC. All heap objects live in the **Heap**, which owns the string intern table and globals.

The GC uses mark-and-sweep, tracing live objects from the stack, call frames, open upvalues, and globals.

The AI primitives are implemented as native bytecode instructions. When the VM executes `generate`, `verify`, `classify`, or `extract`, it makes a synchronous HTTP call to the Cerebras inference API, blocks until the response arrives, and pushes the result onto the stack.

Release binary size: **~500 KB**.

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

The `.scar` file extension is the convention for Scarlet source files.

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

Scarlet is built for educational purposes, heavily inspired by the clox implementation of the Lox language from _Crafting Interpreters_ by Bob Nystrom.
