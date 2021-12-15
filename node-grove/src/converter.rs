use grovedb::{Element, Error};
use neon::prelude::*;
use neon::borrow::Borrow;

pub fn element_to_js_value<'a, C: Context<'a>>(element: Element, cx: &mut C) -> NeonResult<Handle<'a, JsValue>> {
    let js_value = match element {
        Element::Item(item) => {
            let js_element = JsBuffer::external(cx, item.clone());
            js_element.upcast()
        }
        Element::Reference(reference) => {
            let js_array: Handle<JsArray> = cx.empty_array();

            for (index, bytes) in reference.iter().enumerate() {
                let js_buffer = JsBuffer::external(cx, bytes.clone());
                let js_value = js_buffer.as_value(cx);
                js_array.set(cx, index as u32, js_value)?;
            }

            js_array.upcast()
        }
        Element::Tree(tree) => {
            let js_element = JsBuffer::external(cx, tree.clone());
            js_element.upcast()
        }
    };

    NeonResult::Ok(js_value)
}

pub fn js_buffer_to_vec_u8<'a, C: Context<'a>>(js_buffer: Handle<JsBuffer>, cx: &mut C) -> Vec<u8> {
    let guard = cx.lock();
    // let key_buffer = js_buffer.deref();
    let key_memory_view = js_buffer.borrow(&guard);
    let key_slice = key_memory_view.as_slice::<u8>();
    key_slice.to_vec()
}

pub fn js_array_of_buffers_to_vec<'a, C: Context<'a>>(js_array: Handle<JsArray>, cx: &mut C) -> NeonResult<Vec<Vec<u8>>> {
    let buf_vec = js_array.to_vec(cx)?;
    let mut vec: Vec<Vec<u8>> = Vec::new();

    for buf in buf_vec {
        let js_buffer_handle = buf.downcast_or_throw::<JsBuffer, _>(cx)?;
        vec.push(js_buffer_to_vec_u8(js_buffer_handle, cx));
    }

    Ok(vec)
}