use std::{
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use async_ui_core::{
    backend::BackendTrait,
    vnode::{
        node_concrete::{ConcreteNodeVNode, RefNode},
        VNode, VNodeTrait,
    },
};
use pin_project_lite::pin_project;
use web_sys::Node;

mod events;

mod button;
mod checkbox;
mod link;
mod list;
mod radio;
mod text;
mod text_input;
mod view;
pub use button::{button, ButtonProps};
pub use checkbox::{checkbox, CheckboxProps};
pub use link::{link, LinkProps};
pub use list::{list, ListModel, ListProps};
pub use radio::{radio_button, radio_group, RadioGroupProps, RadioProps};
pub use text::text;
pub use text_input::{text_input, TextInputProps};
pub use view::{view, ViewProps};

use crate::backend::Backend;

pin_project! {
    pub struct ElementFuture<F: Future> {
        #[pin]
        future: F,
        inner: ElementFutureInner
    }
}
struct ElementFutureInner {
    node: Node,
    vnodes: Option<MyAndParentVNodes>,
}
struct MyAndParentVNodes {
    my: Rc<VNode<Backend>>,
    parent: Rc<VNode<Backend>>,
}

impl Drop for ElementFutureInner {
    fn drop(&mut self) {
        if let Some(MyAndParentVNodes { parent, .. }) = &self.vnodes {
            parent.del_child_node(Default::default());
        }
    }
}
impl<F: Future> ElementFuture<F> {
    pub fn new(future: F, node: Node) -> Self {
        Self {
            future,
            inner: ElementFutureInner { node, vnodes: None },
        }
    }
}
impl<F: Future> Future for ElementFuture<F> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let vnk = Backend::get_vnode_key();
        let vnodes = this.inner.vnodes.get_or_insert_with(|| {
            let parent_vnode = vnk.with(Clone::clone);
            parent_vnode.add_child_node(this.inner.node.to_owned(), Default::default());
            let parent_context = parent_vnode.get_context_map().clone();
            let my = Rc::new(
                ConcreteNodeVNode::new(
                    RefNode::Parent {
                        parent: this.inner.node.clone(),
                    },
                    parent_context,
                )
                .into(),
            );
            MyAndParentVNodes {
                my,
                parent: parent_vnode,
            }
        });
        vnk.set(&vnodes.my, || this.future.poll(cx))
    }
}
