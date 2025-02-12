use tree_sitter::TreeCursor;

pub fn walk<'a, T, F: Fn(&TreeCursor<'a>) -> T>(cursor: TreeCursor<'a>, map: F) -> impl Iterator<Item = T> + use<'a, T, F> {
    Iter { cursor: Some(cursor), map }
}

pub struct Iter<'a, T, F: Fn(&TreeCursor<'a>) -> T> {
    cursor: Option<TreeCursor<'a>>,
    map: F,
}

impl<'a, T, F: Fn(&TreeCursor<'a>) -> T> Iterator for Iter<'a, T, F> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let cursor = self.cursor.as_mut()?;
        let out = (self.map)(cursor);

        if cursor.goto_first_child() || cursor.goto_next_sibling() {
            return Some(out);
        } 

        while cursor.goto_parent() {
            if cursor.goto_next_sibling() {
                return Some(out);
            }
        }

        self.cursor = None;
        Some(out)
    }
}