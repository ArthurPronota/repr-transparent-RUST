#[repr(transparent)]
struct Wrapped(i32) ;

// Объявление для RUST
// Реальная функция на C: int add(int a, int b)
// Это обертка через FFI 
unsafe extern "C" {
    fn add(x: Wrapped, y: Wrapped) ->i32 ;
}

fn main() {
    let x = Wrapped(10) ;
    let y = Wrapped(20) ;

    println!("res: {}",
        unsafe {
            add(x, y)
        }
    ) ; // Out: res: 30
}
