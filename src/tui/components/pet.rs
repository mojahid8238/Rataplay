use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

pub struct Pet {
    frame_index: usize,
}

impl Pet {
    pub fn new(frame_index: usize) -> Self {
        Self { frame_index }
    }
}

const CAT_FRAMES: &[&str] = &[
    // Frame 0: Sitting, eyes open
    r" /\_/\
( o.o )
 > ^ < ",
    // Frame 1: Eyes closed (Blink)
    r" /\_/\
( -.- )
 > ^ < ",
    // Frame 2: Eyes open
    r" /\_/\
( o.o )
 > ^ < ",
    // Frame 3: Look left
    r" /\_/\
( <.< )
 > ^ < ",
    // Frame 4: Look right
    r" /\_/\
( >.> )
 > ^ < ",
    // Frame 5: Eyes open
    r" /\_/\
( o.o )
 > ^ < ",
];

impl Widget for Pet {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Cycle through frames based on index
        // Use a modulo to loop safely
        let frame_str = CAT_FRAMES[self.frame_index % CAT_FRAMES.len()];
        
        let lines: Vec<&str> = frame_str.split('\n').collect();
        let height = lines.len() as u16;
        let width = lines.iter().map(|l| l.len()).max().unwrap_or(0) as u16;

        // Center the pet in the given area
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;

        for (i, line) in lines.iter().enumerate() {
            if y + i as u16 >= area.bottom() {
                break;
            }
            buf.set_string(
                x,
                y + i as u16,
                line,
                Style::default().fg(Color::Cyan), // Make the cat Cyan
            );
        }
    }
}