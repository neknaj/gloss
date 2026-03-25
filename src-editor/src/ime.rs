use alloc::string::String;
use src_desktop_types::ImeEvent;
use crate::gap_buffer::GapBuffer;
use crate::cursor::Cursor;

pub struct Preedit {
    pub text:            String,
    pub cursor:          Option<(usize, usize)>,
    pub insert_byte_pos: usize,
}

pub struct ImeState {
    pub composing: Option<Preedit>,
}

impl ImeState {
    pub fn new() -> Self { ImeState { composing: None } }
    pub fn is_composing(&self) -> bool { self.composing.is_some() }

    pub fn apply(&mut self, event: ImeEvent, buf: &mut GapBuffer, cursor: &mut Cursor) {
        match event {
            ImeEvent::Start => {
                self.composing = Some(Preedit {
                    text: String::new(),
                    cursor: None,
                    insert_byte_pos: cursor.byte_pos,
                });
            }
            ImeEvent::Update { preedit, cursor: ime_cursor } => {
                if let Some(p) = &mut self.composing {
                    p.text   = preedit;
                    p.cursor = ime_cursor;
                }
            }
            ImeEvent::Commit { text } => {
                if let Some(p) = self.composing.take() {
                    let pos = p.insert_byte_pos;
                    let len = text.len();
                    buf.insert(pos, &text);
                    cursor.byte_pos = pos + len;
                    cursor.sync_line_col(buf);
                }
            }
            ImeEvent::Cancel => {
                self.composing = None;
            }
        }
    }
}

impl Default for ImeState {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (GapBuffer, Cursor, ImeState) {
        (GapBuffer::from_str("hello"), Cursor::new(), ImeState::new())
    }

    #[test] fn start_records_position() {
        let (mut buf, mut cur, mut ime) = setup();
        cur.byte_pos = 3;
        ime.apply(ImeEvent::Start, &mut buf, &mut cur);
        assert!(ime.is_composing());
        assert_eq!(ime.composing.as_ref().unwrap().insert_byte_pos, 3);
    }
    #[test] fn update_sets_preedit_text() {
        let (mut buf, mut cur, mut ime) = setup();
        ime.apply(ImeEvent::Start, &mut buf, &mut cur);
        ime.apply(ImeEvent::Update { preedit: "にほ".into(), cursor: None }, &mut buf, &mut cur);
        assert_eq!(ime.composing.as_ref().unwrap().text, "にほ");
    }
    #[test] fn commit_inserts_into_buffer() {
        let (mut buf, mut cur, mut ime) = setup();
        cur.byte_pos = 5; // end of "hello"
        ime.apply(ImeEvent::Start, &mut buf, &mut cur);
        ime.apply(ImeEvent::Commit { text: "語".into() }, &mut buf, &mut cur);
        assert!(!ime.is_composing());
        assert_eq!(buf.as_str(), "hello語");
    }
    #[test] fn cancel_clears_composing() {
        let (mut buf, mut cur, mut ime) = setup();
        ime.apply(ImeEvent::Start, &mut buf, &mut cur);
        ime.apply(ImeEvent::Cancel, &mut buf, &mut cur);
        assert!(!ime.is_composing());
    }
    #[test] fn commit_moves_cursor_to_end() {
        let (mut buf, mut cur, mut ime) = setup();
        ime.apply(ImeEvent::Start, &mut buf, &mut cur);
        ime.apply(ImeEvent::Commit { text: "ab".into() }, &mut buf, &mut cur);
        assert_eq!(cur.byte_pos, 2); // "ab" = 2 bytes
    }
}
