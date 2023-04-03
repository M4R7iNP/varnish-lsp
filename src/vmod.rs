use goblin::elf::Elf;
use serde_json::{self, Value as SerdeValue};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::{collections::BTreeMap, error::Error};

use crate::varnish_builtins::{Func, Obj, Type};

#[repr(C)]
#[derive(Debug)]
struct VmodDataCStruct {
    vrt_major: u32,
    vrt_minor: u32,
    file_id: *const c_char,
    name: *const c_char,
    func: *const u8,
    func_len: i32,
    proto: *const c_char,
    json: *const c_char,
    abi: *const c_char,
}

#[repr(C)]
#[derive(Debug)]
pub struct VmodData {
    pub vrt_major: usize,
    pub vrt_minor: usize,
    pub file_id: String,
    pub name: String,
    pub func: String,
    pub func_len: usize,
    pub proto: String,
    pub json: String,
    pub abi: String,
    pub scope: Type,
}

#[derive(Debug)]
pub struct VmodFuncArg {
    pub name: String,
    pub input_type: String,
}

#[derive(Debug)]
pub struct VmodFunc {
    pub name: String,
    pub args: Vec<VmodFuncArg>,
    pub ret_type: String,
}

#[derive(Debug)]
pub struct VmodJsonData {
    pub vmod_version: String,
    pub events: Vec<String>,
    pub funcs: Vec<VmodFunc>,
}

fn parse_vmod_json_func(serde_value_arr: &Vec<SerdeValue>) -> Result<Func, Box<dyn Error>> {
    let name = serde_value_arr
        .get(1)
        .ok_or("Missing VMOD func name")?
        .as_str()
        .ok_or("VMOD func name is not string")?
        .to_string();

    let signature_arr = serde_value_arr
        .get(2)
        .ok_or("could not find method signature")?
        .as_array()
        .ok_or("method signature not array")?;

    let ret_types: Vec<String> = signature_arr
        .get(0)
        .ok_or("Missing return type field")?
        .as_array()
        .ok_or("Return type should be array")?
        .iter()
        .map(|ret_type| -> Result<String, Box<dyn Error>> {
            Ok(ret_type
                .as_str()
                .ok_or("Return type is not string")?
                .to_string())
        })
        .filter(|result| result.is_ok())
        .map(|result| result.unwrap())
        .collect();

    let signature = format!(
        "({})",
        signature_arr[3..]
            .iter()
            .map(|arg| -> Result<String, Box<dyn Error>> {
                let arg_arr = arg.as_array().ok_or("Arg signature is not array")?;

                let input_type = arg_arr
                    .get(0)
                    .ok_or("Missing VMOD method arg type")?
                    .as_str()
                    .ok_or("VMOD method arg type should be string")?
                    .to_string();
                let name = arg_arr
                    .get(1)
                    .ok_or("Missing VMOD method arg name")?
                    .as_str()
                    .ok_or("VMOD method arg name should be string")?
                    .to_string();

                Ok(format!("{} {}", input_type, name))
            })
            .filter(|result| result.is_ok())
            .map(|result| result.unwrap())
            .collect::<Vec<String>>()
            .join(", ")
    );

    let ret_type = ret_types.get(0).ok_or("Missing return type")?.as_str();
    let r#return: Option<Box<Type>> = match ret_type {
        "BACKEND" => Some(Box::new(Type::Backend)),
        "STRING" => Some(Box::new(Type::String)),
        "REAL" => Some(Box::new(Type::Number)),
        "INT" => Some(Box::new(Type::Number)),
        "BOOL" => Some(Box::new(Type::Bool)),
        "VOID" => None,
        _ => None,
    };

    Ok(Func {
        name,
        signature: Some(signature),
        ret_type: Some(ret_type.to_string()),
        r#return,
        ..Default::default()
    })
}

pub fn parse_vmod_json(json: &str) -> Result<Type, Box<dyn Error>> {
    let json_parsed: Vec<Vec<SerdeValue>> = serde_json::from_str(&json)?;
    // println!("json test: {:?}", json_parsed);
    /*
    let mut vmod_json_data = VmodJsonData {
        vmod_version: String::new(),
        events: Vec::new(),
        funcs: Vec::new(),
    };
    */

    let mut vmod_obj = Obj {
        name: "".to_string(),
        read_only: true,
        definition: None,
        properties: BTreeMap::new(),
    };

    for row in json_parsed.iter() {
        let row_type = row.get(0).ok_or("empty array")?.as_str();
        if row_type.is_none() {
            continue;
        }

        let row_type = row_type.unwrap();

        match row_type {
            "$VMOD" => {
                /*
                let value = row
                    .get(1)
                    .ok_or("Failed to parse VMOD version")?
                    .as_str()
                    .ok_or("VMOD version is not string")?
                    .to_string();
                vmod_json_data.vmod_version = value;
                */
            }
            "$EVENT" => {
                /*
                let name = row
                    .get(1)
                    .ok_or("Failed to get event name")?
                    .as_str_()
                    .ok_or("Event name is not string")?
                    .to_string();
                vmod_json_data.events.push(name);
                */
            }
            "$FUNC" => {
                let func = parse_vmod_json_func(&row)?;
                // vmod_json_data.funcs.push(func);
                vmod_obj
                    .properties
                    .insert(func.name.clone(), Type::Func(func));
            }
            "$OBJ" => {
                let name = row
                    .get(1)
                    .ok_or("Failed to get obj name")?
                    .as_str()
                    .ok_or("Obj name is not string")?
                    .to_string();

                let mut obj = Obj {
                    name: name.clone(),
                    read_only: true,
                    definition: None,
                    properties: BTreeMap::new(),
                };

                for method_serde_val in row[6..].iter() {
                    let method_arr = method_serde_val.as_array().ok_or("Method is not array")?;
                    let func = parse_vmod_json_func(method_arr)?;
                    obj.properties.insert(func.name.clone(), Type::Func(func));
                }

                let func = Func {
                    name: name.clone(),
                    signature: None,
                    ret_type: Some(name.clone()),
                    definition: None,
                    r#return: Some(Box::new(Type::Obj(obj))),
                };

                vmod_obj.properties.insert(name, Type::Func(func));
            }
            _ => {}
        }
    }

    return Ok(Type::Obj(vmod_obj));
}

/*
pub fn convert_to_varnish_builtin_type(vmod_json_data: VmodJsonData, name: String) -> Type {
    let vmod = Type::Obj(Obj {
        name,
        read_only: true,
        properties: BTreeMap::from_iter(vmod_json_data.funcs.iter().map(|func| {
            (
                func.name.clone(),
                Type::Func(Func {
                    name: func.name.clone(),
                    signature: Some(format!(
                        "({})",
                        func.args
                            .iter()
                            .map(|arg| format!("{} {}", arg.input_type, arg.name))
                            .collect::<Vec<String>>()
                            .join(", ")
                    )),
                    definition: None,
                }),
            )
        })),
        definition: None,
    });

    return vmod;
}
*/

pub async fn read_vmod_lib(vmod_name: String, path: String) -> Result<VmodData, Box<dyn Error>> {
    let file = async_std::fs::read(path).await?;
    let elf = Elf::parse(&file)?;

    let vmod_data_symbol_name = format!("Vmod_{}_Data", vmod_name);
    let (vmd_sym_idx, vmd_sym) = elf
        .dynsyms
        .iter()
        .enumerate()
        .find(|(_, sym)| {
            elf.dynstrtab
                .get_at(sym.st_name)
                .and_then(|sym_name| Some(sym_name == vmod_data_symbol_name))
                .unwrap_or_else(|| false)
        })
        .ok_or("Could not find vmod data symbol")?;

    let sec = &elf
        .section_headers
        .get(vmd_sym.st_shndx)
        .expect("Could not find section");
    let offset = sec.sh_offset as usize + vmd_sym_idx as usize * sec.sh_entsize as usize;

    let vmd_ptr =
        unsafe { std::mem::transmute::<_, *const VmodDataCStruct>(file[offset..].as_ptr()) };
    let vmd = unsafe { &*vmd_ptr };

    let json = CStr::from_bytes_until_nul(&file[(vmd.json as usize)..])?.to_string_lossy();

    let vmod_json_data = parse_vmod_json(&json)?;
    return Ok(VmodData {
        vrt_major: vmd.vrt_major as usize,
        vrt_minor: vmd.vrt_minor as usize,
        name: CStr::from_bytes_until_nul(&file[(vmd.name as usize)..])?
            .to_string_lossy()
            .to_string(),
        file_id: CStr::from_bytes_until_nul(&file[(vmd.file_id as usize)..])?
            .to_string_lossy()
            .to_string(),
        func: CStr::from_bytes_until_nul(&file[(vmd.func as usize)..])?
            .to_string_lossy()
            .to_string(),
        func_len: vmd.func_len as usize,
        proto: CStr::from_bytes_until_nul(&file[(vmd.proto as usize)..])?
            .to_string_lossy()
            .to_string(),
        abi: CStr::from_bytes_until_nul(&file[(vmd.abi as usize)..])?
            .to_string_lossy()
            .to_string(),
        json: json.to_string(),
        scope: vmod_json_data,
    });
}

pub async fn read_vmod_lib_by_name(name: String) -> Result<VmodData, Box<dyn Error>> {
    let path = format!("/usr/lib/varnish-plus/vmods/libvmod_{}.so", name);
    return read_vmod_lib(name, path).await;
}