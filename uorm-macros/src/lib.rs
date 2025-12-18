use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemStruct, ImplItemFn, ItemFn, Expr, ExprAssign, ExprLit, Lit, ReturnType, Type, TypePath};
use syn::parse::Parser;

#[proc_macro_attribute]
pub fn sql(attr: TokenStream, item: TokenStream) -> TokenStream {
    let (namespace, id, database) = parse_args_ts(attr.clone());

    if let Ok(st) = syn::parse::<ItemStruct>(item.clone()) {
        return handle_struct(namespace, st);
    }
    if let Ok(mf) = syn::parse::<ImplItemFn>(item.clone()) {
        return handle_impl_fn(id, database, mf);
    }
    if let Ok(ff) = syn::parse::<ItemFn>(item.clone()) {
        return handle_free_fn(id, database, ff);
    }
    item
}

fn parse_args_ts(attr: TokenStream) -> (Option<String>, Option<String>, Option<String>) {
    let ts: proc_macro2::TokenStream = attr.into();
    let parser = syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated;
    let exprs = parser.parse2(ts).unwrap_or_default();
    let mut namespace = None;
    let mut id = None;
    let mut database = None;
    for e in exprs {
        match e {
            Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) => { id = Some(s.value()); }
            Expr::Assign(ExprAssign { left, right, .. }) => {
                let key = match *left {
                    Expr::Path(p) => p.path.get_ident().map(|i| i.to_string()).unwrap_or_default(),
                    _ => String::new(),
                };
                if let Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) = *right {
                    match key.as_str() {
                        "namespace" => namespace = Some(s.value()),
                        "id" => id = Some(s.value()),
                        "database" => database = Some(s.value()),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    (namespace, id, database)
}

fn handle_struct(namespace: Option<String>, st: ItemStruct) -> TokenStream {
    let name = &st.ident;
    if let Some(ns) = namespace {
        let gen = quote! {
            #st
            impl ::uorm::sql::SqlNamespace for #name {
                fn namespace() -> Option<&'static str> { Some(#ns) }
            }
        };
        gen.into()
    } else {
        let gen = quote! { #st };
        gen.into()
    }
}

fn handle_impl_fn(id: Option<String>, database: Option<String>, mut mf: ImplItemFn) -> TokenStream {
    let id = id.expect("missing sql id");
    let db_name = database.unwrap_or_else(|| "default".to_string());
    let params: Vec<syn::Ident> = mf.sig.inputs.iter().filter_map(|a| {
        if let syn::FnArg::Typed(pt) = a {
            if let syn::Pat::Ident(pi) = &*pt.pat {
                Some(pi.ident.clone())
            } else { None }
        } else { None }
    }).collect();
    let args_tuple = if params.is_empty() {
        quote! { () }
    } else if params.len() == 1 {
        let p = &params[0];
        quote! { #p }
    } else {
        quote! { ( #(#params),* ) }
    };

    let is_async = mf.sig.asyncness.is_some();
    let (setup, ret_expr) = match infer_call_kind(&mf.sig.output) {
        CallKind::Query(elem_ty) => {
            let setup = quote! {
                let __ns = <Self as ::uorm::sql::SqlNamespace>::namespace();
                let __UORM_NAME = match __ns { Some(ns) => format!("{}.{}", ns, #id), None => #id.to_string() };
                let __UORM_SQL = match ::uorm::sql::fetch_sql(&__UORM_NAME) {
                    Ok(s) => s,
                    Err(e) => return Err(e),
                };
                let __UORM_ARGS = #args_tuple;
                const __UORM_DB: &str = #db_name;
                const __UORM_KIND_QUERY: bool = true;
                type __UORM_ELEM = #elem_ty;
            };
            (setup, quote! { exec!() })
        }
        CallKind::Execute => {
            let setup = quote! {
                let __ns = <Self as ::uorm::sql::SqlNamespace>::namespace();
                let __UORM_NAME = match __ns { Some(ns) => format!("{}.{}", ns, #id), None => #id.to_string() };
                let __UORM_SQL = match ::uorm::sql::fetch_sql(&__UORM_NAME) {
                    Ok(s) => s,
                    Err(e) => return Err(e),
                };
                let __UORM_ARGS = #args_tuple;
                const __UORM_DB: &str = #db_name;
                const __UORM_KIND_QUERY: bool = false;
            };
            (setup, quote! { exec!() })
        }
        CallKind::Unit => {
            let setup = quote! {
                let __ns = <Self as ::uorm::sql::SqlNamespace>::namespace();
                let __UORM_NAME = match __ns { Some(ns) => format!("{}.{}", ns, #id), None => #id.to_string() };
                let __UORM_SQL = match ::uorm::sql::fetch_sql(&__UORM_NAME) {
                    Ok(s) => s,
                    Err(e) => { let _ = e; return; }
                };
                let __UORM_ARGS = #args_tuple;
                const __UORM_DB: &str = #db_name;
                const __UORM_KIND_QUERY: bool = false;
            };
            (setup, quote! { exec!() })
        }
    };

    if is_async {
        mf.block = syn::parse_quote! {{
            #setup
            #ret_expr.await
        }};
    } else {
        mf.block = syn::parse_quote! {{
            #setup
            ::tokio::runtime::Handle::current().block_on(async move { #ret_expr.await })
        }};
    }

    quote! { #mf }.into()
}

fn handle_free_fn(id: Option<String>, database: Option<String>, mut ff: ItemFn) -> TokenStream {
    let id = id.expect("missing sql id");
    let db_name = database.unwrap_or_else(|| "default".to_string());
    let params: Vec<syn::Ident> = ff.sig.inputs.iter().filter_map(|a| {
        if let syn::FnArg::Typed(pt) = a {
            if let syn::Pat::Ident(pi) = &*pt.pat {
                Some(pi.ident.clone())
            } else { None }
        } else { None }
    }).collect();
    let args_tuple = if params.is_empty() {
        quote! { () }
    } else if params.len() == 1 {
        let p = &params[0];
        quote! { #p }
    } else {
        quote! { ( #(#params),* ) }
    };
    let is_async = ff.sig.asyncness.is_some();
    let (setup, ret_expr) = match infer_call_kind(&ff.sig.output) {
        CallKind::Query(elem_ty) => {
            let setup = quote! {
                let __UORM_NAME = #id.to_string();
                let __UORM_SQL = match ::uorm::sql::fetch_sql(&__UORM_NAME) {
                    Ok(s) => s,
                    Err(e) => return Err(e),
                };
                let __UORM_ARGS = #args_tuple;
                const __UORM_DB: &str = #db_name;
                const __UORM_KIND_QUERY: bool = true;
                type __UORM_ELEM = #elem_ty;
            };
            (setup, quote! { exec!() })
        }
        CallKind::Execute => {
            let setup = quote! {
                let __UORM_NAME = #id.to_string();
                let __UORM_SQL = match ::uorm::sql::fetch_sql(&__UORM_NAME) {
                    Ok(s) => s,
                    Err(e) => return Err(e),
                };
                let __UORM_ARGS = #args_tuple;
                const __UORM_DB: &str = #db_name;
                const __UORM_KIND_QUERY: bool = false;
            };
            (setup, quote! { exec!() })
        }
        CallKind::Unit => {
            let setup = quote! {
                let __UORM_NAME = #id.to_string();
                let __UORM_SQL = match ::uorm::sql::fetch_sql(&__UORM_NAME) {
                    Ok(s) => s,
                    Err(e) => { let _ = e; return; }
                };
                let __UORM_ARGS = #args_tuple;
                const __UORM_DB: &str = #db_name;
                const __UORM_KIND_QUERY: bool = false;
            };
            (setup, quote! { exec!() })
        }
    };
    if is_async {
        ff.block = syn::parse_quote! {{
            #setup
            #ret_expr.await
        }};
    } else {
        ff.block = syn::parse_quote! {{
            #setup
            ::tokio::runtime::Handle::current().block_on(async move { #ret_expr.await })
        }};
    }
    quote! { #ff }.into()
}

enum CallKind {
    Query(Type),
    Execute,
    Unit,
}

fn infer_call_kind(ret: &ReturnType) -> CallKind {
    match ret {
        ReturnType::Default => CallKind::Unit,
        ReturnType::Type(_, ty) => {
            if let Type::Path(TypePath { path, .. }) = &**ty {
                let segs = &path.segments;
                if segs.len() == 1 && segs[0].ident == "Result" {
                    if let syn::PathArguments::AngleBracketed(args) = &segs[0].arguments {
                        if args.args.len() >= 1 {
                            if let syn::GenericArgument::Type(Type::Path(tp)) = &args.args[0] {
                                let vsegs = &tp.path.segments;
                                if vsegs.len() == 1 && vsegs[0].ident == "Vec" {
                                    if let syn::PathArguments::AngleBracketed(vargs) = &vsegs[0].arguments {
                                        if let Some(syn::GenericArgument::Type(elem)) = vargs.args.first() {
                                            return CallKind::Query(elem.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            CallKind::Execute
        }
    }
}

#[proc_macro]
pub fn exec(_input: TokenStream) -> TokenStream {
    let gen = quote! {
        async move {
            let __pm = ::uorm::pool_manager::global_pool_manager();
            let __client = __pm.get(__UORM_DB).ok_or(::uorm::error::DbError::Connection("database not found".into()))?;
            if __UORM_KIND_QUERY {
                __client.query::<__UORM_ELEM, _>(__UORM_SQL, &__UORM_ARGS).await
            } else {
                __client.execute(__UORM_SQL, &__UORM_ARGS).await
            }
        }
    };
    gen.into()
}
