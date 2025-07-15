use proc_macro::TokenStream;

#[cfg(windows)]
mod windows;

#[cfg(windows)]
#[proc_macro]
pub fn patch_fn(input: TokenStream) -> TokenStream {
  windows::generate_patch(input.into()).into()
}
