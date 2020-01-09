use std::collections::HashMap;
use std::{convert::TryInto, ptr::null_mut};
use z3_sys::*;
use {Context, Datatype, DatatypeAccessor, DatatypeSort, DatatypeVariant, FuncDecl, Sort, Symbol};

pub fn create_datatypes<'ctx>(ds: &[Datatype<'ctx>]) -> Vec<DatatypeSort<'ctx>> {
    let ctx: &'ctx Context = ds[0].ctx;
    let num = ds.len();

    let mut names: Vec<Z3_symbol> = Vec::with_capacity(num);
    let mut out: Vec<Z3_sort> = Vec::with_capacity(num);
    let mut clists: Vec<Z3_constructor_list> = Vec::with_capacity(num);

    for d in ds.iter() {
        names.push(d.name.as_z3_symbol(ctx));
        let num_cs = d.constructors.len();
        let mut cs: Vec<Z3_constructor> = Vec::with_capacity(num_cs);

        for (cname, fs) in &d.constructors {
            let mut rname: String = "is-".to_string();
            rname.push_str(cname);

            let cname_symbol: Z3_symbol = Symbol::String(cname.clone()).as_z3_symbol(ctx);
            let rname_symbol: Z3_symbol = Symbol::String(rname).as_z3_symbol(ctx);

            let num_fs = fs.len();
            let mut field_names: Vec<Z3_symbol> = Vec::with_capacity(num_fs);
            let mut field_sorts: Vec<Z3_sort> = Vec::with_capacity(num_fs);
            let mut sort_refs: Vec<::std::os::raw::c_uint> = Vec::with_capacity(num_fs);

            for (fname, accessor) in fs {
                field_names.push(Symbol::String(fname.clone()).as_z3_symbol(ctx));
                match accessor {
                    DatatypeAccessor::Dtype(datatype) => {
                        field_sorts.push(std::ptr::null_mut());
                        // TODO: sort_refs.push(index of datatype in ds)
                    }
                    DatatypeAccessor::Srt(sort) => {
                        field_sorts.push(sort.z3_sort);
                        sort_refs.push(0);
                    }
                }
            }

            let constructor = unsafe {
                Z3_mk_constructor(
                    ctx.z3_ctx,
                    cname_symbol,
                    rname_symbol,
                    num_fs.try_into().unwrap(),
                    field_names.as_ptr(),
                    field_sorts.as_ptr(),
                    sort_refs.as_mut_ptr(),
                )
            };
            cs.push(constructor);
        }
        let clist = unsafe {
            Z3_mk_constructor_list(ctx.z3_ctx, num_cs.try_into().unwrap(), cs.as_mut_ptr())
        };
        clists.push(clist);
    }
    unsafe {
        Z3_mk_datatypes(
            ctx.z3_ctx,
            num.try_into().unwrap(),
            names.as_ptr(),
            out.as_mut_ptr(),
            clists.as_mut_ptr(),
        )
    };

    let mut datatype_sorts: Vec<DatatypeSort<'ctx>> = Vec::with_capacity(out.len());
    for i in 0..out.len() {
        let s = out[i];
        let d = &ds[i];
        let num_cs = d.constructors.len();

        unsafe {Z3_inc_ref (ctx.z3_ctx, Z3_sort_to_ast(ctx.z3_ctx, s))};
        let sort = Sort { ctx, z3_sort: s };

        let mut variants: Vec<DatatypeVariant<'ctx>> = Vec::with_capacity(num_cs as usize);

        for (j, (cname, fs)) in d.constructors.iter().enumerate() {
            let num_fs = fs.len();
            let constructor_func: Z3_func_decl =
                unsafe { Z3_get_datatype_sort_constructor(ctx.z3_ctx, s, j.try_into().unwrap()) };
            let tester =
                unsafe { Z3_get_datatype_sort_recognizer(ctx.z3_ctx, s, j.try_into().unwrap()) };
            let mut accessors: Vec<Z3_func_decl> = Vec::new();
            accessors.resize(num_fs, null_mut());
            for k in 0..num_fs {
                accessors.push(unsafe {
                    Z3_get_datatype_sort_constructor_accessor(
                        ctx.z3_ctx,
                        s,
                        j.try_into().unwrap(),
                        k.try_into().unwrap(),
                    )
                })
            }

            variants.push(unsafe {
                let constructor = FuncDecl::from_raw(ctx, constructor_func);
                let tester = FuncDecl::from_raw(ctx, tester);
                let accessors = accessors
                    .iter()
                    .map(|f| FuncDecl::from_raw(ctx, *f))
                    .collect();

                DatatypeVariant {
                    constructor,
                    tester,
                    accessors,
                }
            });
        }

        datatype_sorts.push(
            DatatypeSort {
                ctx,
                sort,
                variants
            }
        )
    }

    datatype_sorts
}

impl<'ctx> Datatype<'ctx> {
    pub fn new(ctx: &'ctx Context, name: impl Into<Symbol>) -> Self {
        Self {
            ctx,
            name: name.into(),
            constructors: HashMap::new(),
        }
    }

    pub fn variant(mut self, name: &str, fields: &'ctx [(&str, &DatatypeAccessor<'ctx>)]) {
        let mut accessor_vec: Vec<(String, &DatatypeAccessor<'ctx>)> = Vec::new();
        for (accessor_name, accessor) in fields {
            accessor_vec.push((accessor_name.to_string(), accessor));
        }

        self.constructors.insert(name.to_string(), accessor_vec);
    }

    pub fn finish(self) -> DatatypeSort<'ctx> {
        create_datatypes(&[self]).remove(0)
    }
}
