// Copyright 2015 The Gfx-rs Developers.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Buffer components for PSO macro.

use std::marker::PhantomData;
use gfx_core::{ConstantBufferSlot, Resources, MAX_VERTEX_ATTRIBUTES};
use gfx_core::{handle, pso, shade};
use gfx_core::factory::Phantom;
use gfx_core::format::Format;
use shade::ToUniform;
use super::{DataLink, DataBind, RawDataSet};

pub use gfx_core::pso::{Element, ElemOffset, ElemStride};

/// A trait to be implemented by any struct having the layout described
/// in the graphics API, like a vertex buffer.
pub trait Structure<F> {
    /// Get the layout of an element by name.
    fn query(&str) -> Option<Element<F>>;
}

type AttributeSlotSet = usize;
/// Service struct to simplify the implementations of `VertexBuffer` and `InstanceBuffer`.
pub struct VertexBufferCommon<T, I>(AttributeSlotSet, PhantomData<(T, I)>);
/// Vertex buffer component. Advanced per vertex.
/// - init: `()`
/// - data: `BufferHandle<T>`
pub type VertexBuffer<T> = VertexBufferCommon<T, [(); 0]>;
/// Instance buffer component. Same as the vertex buffer but advances per instance.
pub type InstanceBuffer<T> = VertexBufferCommon<T, [(); 1]>;
/// Constant buffer component.
/// - init: `&str` = name of the buffer
/// - data: `BufferHandle<T>`
pub struct ConstantBuffer<T: Structure<shade::ConstFormat>>(Option<ConstantBufferSlot>, PhantomData<T>);
/// Global (uniform) constant component. Describes a free-standing value passed into
/// the shader, which is not enclosed into any constant buffer. Deprecated in DX10 and higher.
/// - init: `&str` = name of the constant
/// - data: `T` = value
pub struct Global<T: ToUniform>(Option<shade::Location>, PhantomData<T>);


fn match_attribute(_: &shade::AttributeVar, _: Format) -> bool {
    true //TODO
}

impl<'a,
    T: Structure<Format>,
    I: AsRef<[()]> + Default,
> DataLink<'a> for VertexBufferCommon<T, I> {
    type Init = ();
    fn new() -> Self {
        VertexBufferCommon(0, PhantomData)
    }
    fn is_active(&self) -> bool {
        self.0 != 0
    }
    fn link_input(&mut self, at: &shade::AttributeVar, _: &Self::Init) ->
                  Option<Result<pso::AttributeDesc, Format>> {
        T::query(&at.name).map(|el| {
            self.0 |= 1 << (at.slot as AttributeSlotSet);
            if match_attribute(at, el.format) {
                let rate = <I as Default>::default().as_ref().len();
                Ok((el, rate as pso::InstanceRate))
            }else {
                Err(el.format)
            }
        })
    }
}

impl<R: Resources, T, I> DataBind<R> for VertexBufferCommon<T, I> {
    type Data = handle::Buffer<R, T>;
    fn bind_to(&self, out: &mut RawDataSet<R>, data: &Self::Data, man: &mut handle::Manager<R>) {
        let value = Some((man.ref_buffer(data.raw()).clone(), 0));
        for i in 0 .. MAX_VERTEX_ATTRIBUTES {
            if (self.0 & (1<<i)) != 0 {
                out.vertex_buffers.0[i] = value;
            }
        }
    }
}

impl<'a, T: Structure<shade::ConstFormat>>
DataLink<'a> for ConstantBuffer<T> {
    type Init = &'a str;
    fn new() -> Self {
        ConstantBuffer(None, PhantomData)
    }
    fn is_active(&self) -> bool {
        self.0.is_some()
    }
    fn link_constant_buffer(&mut self, cb: &shade::ConstantBufferVar, init: &Self::Init) ->
                            Option<Result<(), shade::ConstFormat>> {
        if &cb.name == *init {
            self.0 = Some(cb.slot);
            Some(Ok(()))
        }else {
            None
        }
    }
}

impl<R: Resources, T: Structure<shade::ConstFormat>>
DataBind<R> for ConstantBuffer<T> {
    type Data = handle::Buffer<R, T>;
    fn bind_to(&self, out: &mut RawDataSet<R>, data: &Self::Data, man: &mut handle::Manager<R>) {
        if let Some(slot) = self.0 {
            let value = Some(man.ref_buffer(data.raw()).clone());
            out.constant_buffers.0[slot as usize] = value;
        }
    }
}

impl<'a, T: ToUniform> DataLink<'a> for Global<T> {
    type Init = &'a str;
    fn new() -> Self {
        Global(None, PhantomData)
    }
    fn is_active(&self) -> bool {
        self.0.is_some()
    }
    fn link_global_constant(&mut self, var: &shade::ConstVar, init: &Self::Init) ->
                            Option<Result<(), shade::UniformValue>> {
        if &var.name == *init {
            //if match_constant(var, ())
            self.0 = Some(var.location);
            Some(Ok(()))
        }else {
            None
        }
    }
}

impl<R: Resources, T: ToUniform> DataBind<R> for Global<T> {
    type Data = T;
    fn bind_to(&self, out: &mut RawDataSet<R>, data: &Self::Data, _: &mut handle::Manager<R>) {
        if let Some(loc) = self.0 {
            let value = data.convert();
            out.global_constants.push((loc, value));
        }
    }
}
