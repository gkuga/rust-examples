struct SelfRef {
    x: u32,
    // ptrは常にxを指していて欲しいが、SelfRefがムーブした瞬間に別のアドレスを指すようになる
    ptr: *const u32,
}

impl SelfRef {
    pub fn new(x: u32) -> SelfRef {
        let mut this = SelfRef {
            x,
            ptr: std::ptr::null(),
        };
        this.ptr = &this.x;

        unsafe {
            println!("x {}", this.x);
            println!("&x {:?}", &this.x as *const u32);
            println!("*ptr {}", *this.ptr);
            println!("ptr {:?}", this.ptr);
        }

        // まだアドレスは変わらないのでテストは成功する
        assert_eq!(&this.x as *const _, this.ptr);

        // ここで値を返した瞬間にxのアドレスが変わり、ptrの値が不正となる
        this
    }
}

fn main() {
    let v = SelfRef::new(0);

    unsafe {
        println!("x {}", v.x);
        println!("&x {:?}", &v.x as *const u32);
        println!("*ptr {}", *v.ptr);
        println!("ptr {:?}", v.ptr);
    }

    // v.xとv.ptrの値が異なるためテスト失敗
    assert_eq!(&v.x as *const _, v.ptr);
}
