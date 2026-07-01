//! Custom iced widgets that the stock 0.14 widget set doesn't provide.
//!
//! [`ContextMenu`] is a click-anchored floating popup: it wraps a trigger
//! element and, while `open`, renders a `menu` element floating just below the
//! trigger as a true overlay (drawn above sibling widgets, correctly clipped
//! and positioned even inside a `scrollable`). Clicking anywhere outside the
//! menu emits `on_dismiss`. iced has no built-in context menu, so we implement
//! the `Widget` + `Overlay` traits directly (modelled on the stock `tooltip`).

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{self, Widget};
use iced::advanced::{mouse, Clipboard, Shell};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

/// A trigger element with an attached floating menu shown while `open`.
pub struct ContextMenu<'a, Message, Theme, Renderer> {
    base: Element<'a, Message, Theme, Renderer>,
    menu: Element<'a, Message, Theme, Renderer>,
    open: bool,
    on_dismiss: Message,
    /// Vertical gap between the trigger and the floating menu.
    gap: f32,
}

impl<'a, Message, Theme, Renderer> ContextMenu<'a, Message, Theme, Renderer> {
    pub fn new(
        base: impl Into<Element<'a, Message, Theme, Renderer>>,
        menu: impl Into<Element<'a, Message, Theme, Renderer>>,
        open: bool,
        on_dismiss: Message,
    ) -> Self {
        Self {
            base: base.into(),
            menu: menu.into(),
            open,
            on_dismiss,
            gap: 2.0,
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for ContextMenu<'_, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: renderer::Renderer,
{
    fn children(&self) -> Vec<widget::Tree> {
        vec![
            widget::Tree::new(&self.base),
            widget::Tree::new(&self.menu),
        ]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[self.base.as_widget(), self.menu.as_widget()]);
    }

    fn size(&self) -> Size<Length> {
        self.base.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.base.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.base
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.base.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.base.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.base.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.base.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            operation,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let mut children = tree.children.iter_mut();
        let base_tree = children.next().unwrap();
        let menu_tree = children.next().unwrap();

        let base_overlay = self.base.as_widget_mut().overlay(
            base_tree,
            layout,
            renderer,
            viewport,
            translation,
        );

        let menu_overlay = if self.open {
            let b = layout.bounds();
            let anchor = Rectangle {
                x: b.x + translation.x,
                y: b.y + translation.y,
                width: b.width,
                height: b.height,
            };
            Some(overlay::Element::new(Box::new(Menu {
                menu: &mut self.menu,
                tree: menu_tree,
                anchor,
                on_dismiss: self.on_dismiss.clone(),
                gap: self.gap,
            })))
        } else {
            None
        };

        if base_overlay.is_some() || menu_overlay.is_some() {
            Some(
                overlay::Group::with_children(
                    base_overlay.into_iter().chain(menu_overlay).collect(),
                )
                .overlay(),
            )
        } else {
            None
        }
    }
}

impl<'a, Message, Theme, Renderer>
    From<ContextMenu<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    fn from(menu: ContextMenu<'a, Message, Theme, Renderer>) -> Self {
        Element::new(menu)
    }
}

struct Menu<'a, 'b, Message, Theme, Renderer> {
    menu: &'b mut Element<'a, Message, Theme, Renderer>,
    tree: &'b mut widget::Tree,
    anchor: Rectangle,
    on_dismiss: Message,
    gap: f32,
}

impl<Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for Menu<'_, '_, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: renderer::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let limits = layout::Limits::new(Size::ZERO, bounds);
        let node = self.menu.as_widget_mut().layout(self.tree, renderer, &limits);
        let size = node.size();

        // Anchor below-left of the trigger; flip above / clamp on overflow.
        let mut x = self.anchor.x;
        let mut y = self.anchor.y + self.anchor.height + self.gap;

        if x + size.width > bounds.width {
            x = (bounds.width - size.width).max(0.0);
        }
        if y + size.height > bounds.height {
            // Not enough room below — place it above the trigger instead.
            y = (self.anchor.y - size.height - self.gap).max(0.0);
        }

        node.translate(Vector::new(x.max(0.0), y.max(0.0)))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.menu.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &Rectangle::with_size(Size::INFINITE),
        );
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        // A press outside the menu dismisses it (and is swallowed so it doesn't
        // also trigger whatever is underneath).
        if let Event::Mouse(mouse::Event::ButtonPressed(_))
        | Event::Touch(iced::touch::Event::FingerPressed { .. }) = event
        {
            if cursor.position_over(layout.bounds()).is_none() {
                shell.publish(self.on_dismiss.clone());
                shell.capture_event();
                return;
            }
        }

        self.menu.as_widget_mut().update(
            self.tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &Rectangle::with_size(Size::INFINITE),
        );
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.menu.as_widget().mouse_interaction(
            self.tree,
            layout,
            cursor,
            &Rectangle::with_size(Size::INFINITE),
            renderer,
        )
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.menu
            .as_widget_mut()
            .operate(self.tree, layout, renderer, operation);
    }

    fn overlay<'c>(
        &'c mut self,
        layout: Layout<'c>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'c, Message, Theme, Renderer>> {
        self.menu.as_widget_mut().overlay(
            self.tree,
            layout,
            renderer,
            &Rectangle::with_size(Size::INFINITE),
            Vector::ZERO,
        )
    }
}

/// Convenience constructor.
pub fn context_menu<'a, Message, Theme, Renderer>(
    base: impl Into<Element<'a, Message, Theme, Renderer>>,
    menu: impl Into<Element<'a, Message, Theme, Renderer>>,
    open: bool,
    on_dismiss: Message,
) -> ContextMenu<'a, Message, Theme, Renderer> {
    ContextMenu::new(base, menu, open, on_dismiss)
}
