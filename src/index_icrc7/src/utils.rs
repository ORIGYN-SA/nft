use std::borrow::Cow;

pub fn trace<'a>(msg: impl Into<Cow<'a, str>>) {
    let msg: Cow<'a, str> = msg.into();

    unsafe {
        ic0::debug_print(msg.as_ptr() as usize, msg.len());
    }
    ic_cdk::println!("{}", msg);
}
