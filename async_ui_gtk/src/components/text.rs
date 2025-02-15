use glib::Cast;
use observables::{ObservableAs, ObservableAsExt};

use crate::widget::{WidgetOp, WrappedWidget};

use super::ElementFuture;

pub async fn text<'c>(text: &'c dyn ObservableAs<str>) {
    let node = gtk::Label::new(None);
    let widget: gtk::Widget = node.clone().upcast();
    ElementFuture::new(
        async {
            loop {
                node.set_label(&*text.borrow_observable_as());
                text.until_change().await;
            }
        },
        WrappedWidget {
            widget: widget.clone(),
            inner_widget: widget.upcast(),
            op: WidgetOp::NoChild,
        },
    )
    .await;
}
