use quote::{format_ident, quote};
use unsynn::{IParse, ToTokens, TokenStream};
use unsynn_rust::FnSignature;

pub(crate) fn ctest(input: TokenStream, item: TokenStream) -> TokenStream {
  let mut function: FnSignature = match item.to_token_iter().parse() {
    Ok(func) => func,
    Err(err) => panic!("Failed to parse: {err}"),
  };

  let original_test_name = function.name.clone();
  let test_name_str = format!("::{original_test_name}");
  let static_ident = format_ident!("TEST_{original_test_name}");
  let test_impl_ident = format_ident!("{original_test_name}_impl");
  function.name.clone_from(&test_impl_ident);
  let function = function.to_token_stream();
  quote! {
    #[linkme::distributed_slice(#input)]
    static #static_ident: (&'static str, fn()) = (concat!(module_path!(), #test_name_str), #test_impl_ident);

    #function

    #[test]
    fn #original_test_name() {
      #test_impl_ident()
    }
  }
}
