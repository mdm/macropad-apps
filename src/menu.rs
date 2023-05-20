use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{MonoFont, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::PixelColor,
    prelude::{Point, Size},
    primitives::{Primitive, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text},
    Drawable,
};

pub struct Menu<'a, C: PixelColor> {
    items: &'a [&'a str],
    selected: usize,
    caption_offset: usize,
    window_start: usize,
    window_len: usize,
    font: &'a MonoFont<'a>,
    normal_text_style: MonoTextStyle<'a, C>,
    inverted_text_style: MonoTextStyle<'a, C>,
    selection_style: PrimitiveStyle<C>,
    clear_style: PrimitiveStyle<C>,
}

impl<'a, C: PixelColor> Menu<'a, C> {
    pub fn new(
        items: &'a [&str],
        height: u32,
        font: &'a MonoFont,
        bg_color: C,
        fg_color: C,
    ) -> Self {
        let font_height = font.character_size.height;
        let window_len = (height / font_height) as usize;

        let normal_text_style = MonoTextStyleBuilder::new()
            .font(font)
            .text_color(fg_color)
            .background_color(bg_color)
            .build();

        let inverted_text_style = MonoTextStyleBuilder::new()
            .font(font)
            .text_color(bg_color)
            .background_color(fg_color)
            .build();

        let selection_style = PrimitiveStyleBuilder::new()
            .stroke_color(fg_color)
            .fill_color(fg_color)
            .build();

        let clear_style = PrimitiveStyleBuilder::new()
            .stroke_color(bg_color)
            .fill_color(bg_color)
            .build();

        Menu {
            items,
            selected: 0,
            caption_offset: 0,
            window_start: 0,
            window_len,
            font,
            normal_text_style,
            inverted_text_style,
            selection_style,
            clear_style,
        }
    }

    pub fn select_item(&mut self, item: usize) {
        if item >= self.items.len() {
            return;
        }

        self.selected = item;
        self.caption_offset = 0;

        while self.selected < self.window_start {
            self.window_start -= 1;
        }

        while self.selected >= self.window_start + self.window_len {
            self.window_start += 1;
        }
    }

    pub fn scroll_item(&mut self, offset: usize) {
        self.caption_offset = offset;
    }
}

impl<C: PixelColor> Drawable for Menu<'_, C> {
    type Color = C;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let font_width = self.font.character_size.width;
        let font_height = self.font.character_size.height;
        let target_width = target.bounding_box().columns().len() as u32;

        target
            .bounding_box()
            .into_styled(self.clear_style)
            .draw(target)?;

        for i in self.window_start..self.window_len {
            if i - self.window_start >= self.items.len() {
                break;
            }

            let rectangle_position =
                Point::new(0, (font_height * (i - self.window_start) as u32) as i32);
            let rectangle_size = Size::new(target_width, font_height);
            let text_position = rectangle_position + Point::new((font_width / 2) as i32, 0);
            if i - self.window_start == self.selected {
                Rectangle::new(rectangle_position, rectangle_size)
                    .into_styled(self.selection_style)
                    .draw(target)?;

                Text::with_baseline(
                    self.items[i - self.window_start],
                    // self.items[i],
                    text_position,
                    self.inverted_text_style,
                    Baseline::Top,
                )
                .draw(target)?;
            } else {
                Text::with_baseline(
                    self.items[i - self.window_start],
                    // self.items[i],
                    text_position,
                    self.normal_text_style,
                    Baseline::Top,
                )
                .draw(target)?;
            }
        }

        Ok(())
    }
}
