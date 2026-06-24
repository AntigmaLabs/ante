use std::collections::VecDeque;

/// Byte buffer that preserves the beginning and end, dropping the middle.
#[derive(Debug, Clone)]
pub struct HeadTailBuffer {
    max_bytes: usize,
    head: VecDeque<Vec<u8>>,
    tail: VecDeque<Vec<u8>>,
    head_bytes: usize,
    tail_bytes: usize,
    omitted_bytes: usize,
}

impl HeadTailBuffer {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            head: VecDeque::new(),
            tail: VecDeque::new(),
            head_bytes: 0,
            tail_bytes: 0,
            omitted_bytes: 0,
        }
    }

    pub fn push_chunk(&mut self, chunk: Vec<u8>) {
        if chunk.is_empty() {
            return;
        }

        if self.max_bytes == 0 {
            self.omitted_bytes += chunk.len();
            return;
        }

        let mut chunk = chunk;
        let head_budget = self.max_bytes / 2;
        let tail_budget = self.max_bytes - head_budget;

        if self.head_bytes < head_budget {
            let to_head = (head_budget - self.head_bytes).min(chunk.len());
            let head_part = chunk.drain(..to_head).collect::<Vec<_>>();
            self.head_bytes += head_part.len();
            self.head.push_back(head_part);
        }

        if chunk.is_empty() {
            return;
        }

        if tail_budget == 0 {
            self.omitted_bytes += chunk.len();
            return;
        }

        if chunk.len() > tail_budget {
            self.omitted_bytes += chunk.len() - tail_budget;
            let split_at = chunk.len() - tail_budget;
            chunk = chunk.split_off(split_at);

            self.omitted_bytes += self.tail_bytes;
            self.tail.clear();
            self.tail_bytes = 0;
        } else {
            self.trim_tail_for(chunk.len(), tail_budget);
        }

        self.tail_bytes += chunk.len();
        self.tail.push_back(chunk);
    }

    pub fn snapshot(&self) -> Vec<Vec<u8>> {
        let mut out = Vec::with_capacity(self.head.len() + self.tail.len());
        out.extend(self.head.iter().cloned());
        out.extend(self.tail.iter().cloned());
        out
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.retained_bytes());
        self.append_retained_to(&mut out, None);
        out
    }

    pub fn to_bytes_with_omission_marker(&self, marker: &[u8]) -> Vec<u8> {
        let marker_bytes = if self.omitted_bytes > 0 { marker.len() } else { 0 };
        let mut out = Vec::with_capacity(self.retained_bytes() + marker_bytes);
        self.append_retained_to(&mut out, Some(marker));

        out
    }

    fn append_retained_to(&self, out: &mut Vec<u8>, marker: Option<&[u8]>) {
        for chunk in &self.head {
            out.extend_from_slice(chunk);
        }
        if self.omitted_bytes > 0
            && let Some(marker) = marker
        {
            out.extend_from_slice(marker);
        }
        for chunk in &self.tail {
            out.extend_from_slice(chunk);
        }
    }

    /// Return up to `max_bytes` from the end of the retained content
    /// (head followed by tail), preserving order. Avoids allocating the full
    /// buffer when only a suffix is needed.
    pub fn suffix_bytes(&self, max_bytes: usize) -> Vec<u8> {
        let take = max_bytes.min(self.retained_bytes());
        if take == 0 {
            return Vec::new();
        }

        let mut out = vec![0u8; take];
        let mut written = 0;
        for chunk in self.tail.iter().rev().chain(self.head.iter().rev()) {
            if written == take {
                break;
            }
            let remaining = take - written;
            let n = chunk.len().min(remaining);
            let chunk_start = chunk.len() - n;
            let dst_end = take - written;
            let dst_start = dst_end - n;
            out[dst_start..dst_end].copy_from_slice(&chunk[chunk_start..]);
            written += n;
        }

        out
    }

    pub fn drain(&mut self) -> Vec<Vec<u8>> {
        let mut out = Vec::with_capacity(self.head.len() + self.tail.len());
        while let Some(chunk) = self.head.pop_front() {
            out.push(chunk);
        }
        while let Some(chunk) = self.tail.pop_front() {
            out.push(chunk);
        }

        self.clear();
        out
    }

    pub fn drain_into(&mut self, target: &mut HeadTailBuffer) {
        while let Some(chunk) = self.head.pop_front() {
            target.push_chunk(chunk);
        }
        while let Some(chunk) = self.tail.pop_front() {
            target.push_chunk(chunk);
        }

        self.clear();
    }

    fn clear(&mut self) {
        self.head.clear();
        self.tail.clear();
        self.head_bytes = 0;
        self.tail_bytes = 0;
        self.omitted_bytes = 0;
    }

    pub fn retained_bytes(&self) -> usize {
        self.head_bytes + self.tail_bytes
    }

    pub fn omitted_bytes(&self) -> usize {
        self.omitted_bytes
    }

    /// Length in bytes of the retained head (the split point: `to_bytes()` is
    /// the head followed by the tail, with `omitted_bytes` dropped between).
    pub fn head_bytes(&self) -> usize {
        self.head_bytes
    }

    fn trim_tail_for(&mut self, incoming_bytes: usize, tail_budget: usize) {
        while self.tail_bytes + incoming_bytes > tail_budget {
            let overflow = self.tail_bytes + incoming_bytes - tail_budget;
            let Some(front) = self.tail.front_mut() else {
                break;
            };

            if front.len() <= overflow {
                let removed = front.len();
                self.tail.pop_front();
                self.tail_bytes -= removed;
                self.omitted_bytes += removed;
                continue;
            }

            front.drain(..overflow);
            self.tail_bytes -= overflow;
            self.omitted_bytes += overflow;
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HeadTailBuffer;
    use pretty_assertions::assert_eq;

    #[test]
    fn retains_all_output_when_under_budget() {
        let mut buffer = HeadTailBuffer::new(16);
        buffer.push_chunk(b"hello".to_vec());
        buffer.push_chunk(b" world".to_vec());

        assert_eq!(buffer.to_bytes(), b"hello world".to_vec());
        assert_eq!(buffer.retained_bytes(), 11);
        assert_eq!(buffer.omitted_bytes(), 0);
    }

    #[test]
    fn preserves_head_and_tail_when_over_budget() {
        let mut buffer = HeadTailBuffer::new(12);
        buffer.push_chunk(b"abcdef".to_vec());
        buffer.push_chunk(b"ghij".to_vec());
        buffer.push_chunk(b"klmnop".to_vec());

        assert_eq!(buffer.to_bytes(), b"abcdefklmnop".to_vec());
        assert_eq!(buffer.retained_bytes(), 12);
        assert_eq!(buffer.omitted_bytes(), 4);
    }

    #[test]
    fn large_chunk_keeps_only_tail_suffix() {
        let mut buffer = HeadTailBuffer::new(10);
        buffer.push_chunk(b"abcde".to_vec());
        buffer.push_chunk(b"0123456789".to_vec());

        assert_eq!(buffer.to_bytes(), b"abcde56789".to_vec());
        assert_eq!(buffer.retained_bytes(), 10);
        assert_eq!(buffer.omitted_bytes(), 5);
    }

    #[test]
    fn omission_marker_is_inserted_between_head_and_tail() {
        let mut buffer = HeadTailBuffer::new(12);
        buffer.push_chunk(b"abcdef".to_vec());
        buffer.push_chunk(b"ghij".to_vec());
        buffer.push_chunk(b"klmnop".to_vec());

        assert_eq!(
            buffer.to_bytes_with_omission_marker(b"\n...[truncated]...\n"),
            b"abcdef\n...[truncated]...\nklmnop".to_vec()
        );
    }

    #[test]
    fn omission_marker_is_not_inserted_without_omitted_bytes() {
        let mut buffer = HeadTailBuffer::new(12);
        buffer.push_chunk(b"abcdef".to_vec());

        assert_eq!(
            buffer.to_bytes_with_omission_marker(b"\n...[truncated]...\n"),
            b"abcdef".to_vec()
        );
    }

    #[test]
    fn suffix_bytes_returns_last_n_across_chunks() {
        let mut buffer = HeadTailBuffer::new(64);
        buffer.push_chunk(b"abc".to_vec());
        buffer.push_chunk(b"defgh".to_vec());
        buffer.push_chunk(b"ij".to_vec());

        assert_eq!(buffer.suffix_bytes(0), Vec::<u8>::new());
        assert_eq!(buffer.suffix_bytes(1), b"j".to_vec());
        assert_eq!(buffer.suffix_bytes(4), b"fghij".to_vec()[1..].to_vec());
        assert_eq!(buffer.suffix_bytes(100), b"abcdefghij".to_vec());
    }

    #[test]
    fn suffix_bytes_spans_head_when_tail_is_short() {
        let mut buffer = HeadTailBuffer::new(12);
        buffer.push_chunk(b"abcdef".to_vec());
        buffer.push_chunk(b"ghij".to_vec());
        buffer.push_chunk(b"klmnop".to_vec());

        // Retained content is "abcdef" ++ "klmnop"; suffix should walk into the head.
        assert_eq!(buffer.suffix_bytes(8), b"efklmnop".to_vec());
    }

    #[test]
    fn zero_budget_omits_everything() {
        let mut buffer = HeadTailBuffer::new(0);
        buffer.push_chunk(b"abcdef".to_vec());

        assert_eq!(buffer.to_bytes(), Vec::<u8>::new());
        assert_eq!(buffer.retained_bytes(), 0);
        assert_eq!(buffer.omitted_bytes(), 6);
    }

    #[test]
    fn drain_returns_chunks_and_resets_state() {
        let mut buffer = HeadTailBuffer::new(8);
        buffer.push_chunk(b"abcd".to_vec());
        buffer.push_chunk(b"efgh".to_vec());

        let drained = buffer.drain();
        let drained_bytes = drained.concat();
        assert_eq!(drained_bytes, b"abcdefgh".to_vec());

        assert_eq!(buffer.retained_bytes(), 0);
        assert_eq!(buffer.omitted_bytes(), 0);
        assert_eq!(buffer.snapshot(), Vec::<Vec<u8>>::new());
    }

    #[test]
    fn drain_into_moves_chunks_without_intermediate_snapshot() {
        let mut source = HeadTailBuffer::new(8);
        let mut target = HeadTailBuffer::new(16);
        source.push_chunk(b"abcd".to_vec());
        source.push_chunk(b"efgh".to_vec());

        source.drain_into(&mut target);

        assert_eq!(target.to_bytes(), b"abcdefgh".to_vec());
        assert_eq!(source.retained_bytes(), 0);
        assert_eq!(source.omitted_bytes(), 0);
        assert_eq!(source.snapshot(), Vec::<Vec<u8>>::new());
    }
}
