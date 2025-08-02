use proc_macro::TokenStream;

mod test_collector;
#[cfg(windows)]
mod windows;

#[cfg(windows)]
#[proc_macro]
pub fn patch_fn(input: TokenStream) -> TokenStream {
  windows::generate_patch(input.into()).into()
}

#[proc_macro_attribute]
pub fn ctest(input: TokenStream, item: TokenStream) -> TokenStream {
  test_collector::ctest(input.into(), item.into()).into()
}
