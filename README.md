# `#[repr(transparent)]` в RUST

## Что говорит ваш код

```rust
#[repr(transparent)]
struct Wrapped(i32);

unsafe extern "C" {
    fn add(x: Wrapped, y: Wrapped) -> i32;
}

fn main() {
    let x = Wrapped(10);
    let y = Wrapped(20);
    println!("res: {}", unsafe { add(x, y) }); // 30
}
```

C-функция:

```c
int add(int a, int b) {
    return a + b;
}
```

Rust объявляет её как `add(Wrapped, Wrapped) -> i32`, хотя в C она принимает `int`. Это работает **только благодаря `#[repr(transparent)]`**.

---

## 🔹 Что делает `#[repr(transparent)]`

Атрибут говорит компилятору:

> «У этой структуры **ровно одно не-ZST поле** (или несколько, но у всех кроме одного — zero-sized). Раскладка в памяти и ABI структуры **идентичны** этому единственному значащему полю».

Для:

```rust
#[repr(transparent)]
struct Wrapped(i32);
```

- Размер `Wrapped` = размер `i32` = **4 байта**.
- Выравнивание = выравнивание `i32` = **4 байта**.
- ABI при передаче через FFI = ABI `i32`.

То есть `Wrapped` **неотличим от `i32`** на уровне машинного кода — как будто обёртки и нет.

---

## 🔹 Почему это критично в вашем примере

### Без `#[repr(transparent)]`

Структура `Wrapped` по умолчанию (`repr(Rust)`) имеет **неопределённый** layout:

```rust
struct Wrapped(i32);   // без repr
```

- Компилятор **вправе** переупорядочивать поля, добавлять padding, менять размер.
- ABI при передаче через FFI — **не гарантирован**.
- C-функция `int add(int, int)` **не знает** про `Wrapped`. Передача `Wrapped` в C — undefined behavior.

### С `#[repr(transparent)]`

```rust
#[repr(transparent)]
struct Wrapped(i32);
```

Компилятор **обязан** сделать layout идентичным `i32`. В ABI:

- `Wrapped` передаётся как `i32` (в регистре, как обычный `int`).
- C-функция `add(int, int)` получает **ровно два `int`** и возвращает `int`.
- Rust видит `add(Wrapped, Wrapped) -> i32`, но на уровне машинного кода всё совпадает.

**Именно поэтому** вызов работает:

```rust
add(x, y)   // Rust: Wrapped, Wrapped
            // C:    int,     int
            // → ABI идентичен → 30
```

---

## 🔹 Требования к `#[repr(transparent)]`

Атрибут **нельзя** поставить на любую структуру. Правила:

| Требование | Пример |
|-----------|--------|
| Ровно **одно** поле с ненулевым размером | `struct W(i32);` ✅ |
| Остальные поля могут быть только **ZST** (zero-sized types) | `struct W(i32, PhantomData<u8>);` ✅ |
| Поле должно быть **не-ZST** | `struct W(PhantomData<u8>);` ❌ |
| Нельзя на enum с данными | `enum E { A(i32) }` ❌ |
| Нельзя на tuple с несколькими полями | `struct W(i32, i32);` ❌ |

Пример с `PhantomData` (ZST):

```rust
use std::marker::PhantomData;

#[repr(transparent)]
struct Wrapped<T>(i32, PhantomData<T>);
// Размер = 4, layout = i32, T «фантомный»
```

Это полезно, когда хочется параметризовать тип, но данные — только `i32`.

---

## 🔹 Где применяется

### 1. Newtype-паттерн для FFI

Самое частое применение — обернуть примитив в новый тип, не меняя ABI:

```rust
#[repr(transparent)]
struct FileDescriptor(i32);

extern "C" {
    fn close(fd: FileDescriptor) -> i32;   // в C: int close(int)
}
```

Плюсы:

- Типобезопасность в Rust: нельзя случайно передать «просто `i32`» вместо дескриптора.
- ABI не меняется: C видит обычный `int`.

### 2. Обёртки над указателями

```rust
#[repr(transparent)]
struct Handle(*mut c_void);
```

Указатель передаётся в C как есть, но в Rust это отдельный тип.

### 3. Interior mutability

`Cell<T>` и `RefCell<T>` не совсем `repr(transparent)`, но идея похожа: обёртка не меняет layout. А вот `UnsafeCell<T>` — **да**, `repr(transparent)`:

```rust
#[repr(transparent)]
pub struct UnsafeCell<T>(T);
```

Поэтому `UnsafeCell<T>` имеет тот же размер и выравнивание, что `T`.

### 4. Типы-единицы измерения

```rust
#[repr(transparent)]
struct Meters(f64);

#[repr(transparent)]
struct Feet(f64);

fn walk(d: Meters) { ... }
// walk(Feet(3.0)) — ошибка компиляции, хотя оба f64
```

Типобезопасность без накладных расходов.

### 5. `NonNull<T>`

Стандартный `NonNull<T>` — `repr(transparent)` над `*const T`:

```rust
#[repr(transparent)]
pub struct NonNull<T: ?Sized> {
    pointer: *const T,
}
```

Поэтому `NonNull<T>` и `*const T` взаимозаменяемы по ABI.

---

## 🔹 Сравнение с другими `repr`

| Атрибут | Что делает | Пример |
|---------|-----------|--------|
| `repr(Rust)` (по умолчанию) | layout не определён, компилятор оптимизирует | `struct S(i32, i32);` |
| `repr(C)` | layout как в C: поля по порядку, с padding | `struct S { a: i32, b: i32 }` |
| **`repr(transparent)`** | layout = layout единственного значащего поля | `struct W(i32);` |
| `repr(packed)` | без padding | `struct S(u8, u32);` — 5 байт |
| `repr(align(N))` | выравнивание N | `struct S(u8) align(16);` |

**Ключевое различие** `repr(C)` и `repr(transparent)`:

- `repr(C)` **фиксирует** порядок полей, но **не** делает структуру идентичной одному полю.
- `repr(transparent)` гарантирует **полную идентичность** одному полю.

Пример:

```rust
#[repr(C)]
struct A(i32, i32);   // размер 8

#[repr(transparent)]
struct B(i32);        // размер 4, ABI = i32
```

---

## 🔹 Ваш пример: цепочка целиком

```
┌────────────────────────────────────────────────────────────┐
│  main.rs (Rust)                                            │
│                                                            │
│  #[repr(transparent)]                                      │
│  struct Wrapped(i32);          ← layout = i32, ABI = i32   │
│                                                            │
│  unsafe extern "C" {                                       │
│      fn add(x: Wrapped, y: Wrapped) -> i32;                │
│  }                                                         │
│                                                            │
│  add(Wrapped(10), Wrapped(20))                             │
│      │                                                     │
│      └─► на уровне ABI: add(int, int)                      │
└──────────────────────┬─────────────────────────────────────┘
                       │
                       ▼
┌────────────────────────────────────────────────────────────┐
│  add.c (C)                                                 │
│                                                            │
│  int add(int a, int b) { return a + b; }                   │
│                                                            │
│  → 30                                                      │
└────────────────────────────────────────────────────────────┘
```

`#[repr(transparent)]` — это **мост** между Rust-типом `Wrapped` и C-типом `int`. Без него ABI не совпал бы, и вызов был бы UB.

---

## 🔹 Что было бы без `#[repr(transparent)]`

Скорее всего, **работало бы** на практике (в текущих реализациях Rust `struct W(i32)` часто имеет layout `i32`), но это:

- **не гарантировано** спецификацией;
- может **сломаться** при смене версии компилятора, оптимизаций, target’а;
- формально — **UB** при передаче через FFI.

С `#[repr(transparent)]` — **гарантировано** и **безопасно** (в смысле ABI).

---

## 🔹 Итог

| Вопрос | Ответ |
|--------|-------|
| Что такое `#[repr(transparent)]`? | Атрибут: layout и ABI структуры = layout и ABI её единственного значащего поля |
| Зачем нужен? | Чтобы обёртка (newtype) не меняла ABI при передаче через FFI |
| Требования | Ровно одно ненулевое поле; остальные — ZST |
| Где применяется | FFI, newtype, обёртки над указателями, `UnsafeCell`, `NonNull`, единицы измерения |
| Что даёт в вашем примере | `Wrapped` передаётся в C как `int`, поэтому `add(Wrapped, Wrapped)` совпадает с `add(int, int)` |
| Что без него? | Формально UB, хотя на практике может работать |

**Ключевая мысль:** `#[repr(transparent)]` — это **обещание компилятору и программисту**, что обёртка «прозрачна» для ABI. В FFI это позволяет безопасно передавать newtype-обёртки туда, где C ожидает примитив, без изменения C-кода.
