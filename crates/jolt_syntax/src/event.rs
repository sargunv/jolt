use crate::RawSyntaxKind;

/// Sentinel stored in a start event when a node has no forward parent.
pub(crate) const NO_FORWARD_PARENT: u32 = 0;

/// A parser event consumed by the shared syntax tree builder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    /// Starts a syntax node with the supplied raw kind.
    Start {
        /// The node kind.
        kind: RawSyntaxKind,
        /// Offset to a later start-node event that should wrap this node.
        /// Zero means that this node has no forward parent. Every real offset
        /// is positive because a forward parent always starts at a later event.
        forward_parent: u32,
    },
    /// Consumes the next token from the token source.
    Token,
    /// Finishes the current syntax node.
    Finish,
    /// A placeholder event reserved by parser marker APIs.
    Tombstone,
    /// A forward-parent start already consumed by tree construction.
    #[doc(hidden)]
    Consumed,
}

/// A placeholder for a syntax node whose kind is not known yet.
#[derive(Debug, Eq, PartialEq)]
pub struct Marker {
    position: usize,
}

impl Marker {
    /// Reserves a start-node slot in the event stream.
    pub fn new(events: &mut Vec<Event>) -> Self {
        let position = events.len();
        events.push(Event::Tombstone);

        Self { position }
    }

    /// Completes this marker as a syntax node of `kind`.
    ///
    /// # Panics
    ///
    /// Panics if this marker's event slot is no longer a tombstone.
    pub fn complete(self, events: &mut Vec<Event>, kind: RawSyntaxKind) -> CompletedMarker {
        let event = events
            .get_mut(self.position)
            .expect("marker position must exist in event stream");
        assert!(
            matches!(event, Event::Tombstone),
            "marker position must contain a tombstone event"
        );

        *event = Event::start_node(kind);
        events.push(Event::Finish);

        CompletedMarker {
            position: self.position,
            kind,
        }
    }

    /// Abandons this marker and removes its placeholder from the event stream.
    ///
    /// # Panics
    ///
    /// Panics if this marker is not the latest event or if its event slot is no
    /// longer a tombstone.
    pub fn abandon(self, events: &mut Vec<Event>) {
        assert_eq!(
            self.position.checked_add(1),
            Some(events.len()),
            "only the latest marker can be abandoned"
        );
        let event = events
            .pop()
            .expect("marker position must exist in event stream");
        assert!(
            matches!(event, Event::Tombstone),
            "marker position must contain a tombstone event"
        );
    }
}

/// A completed parser marker.
#[derive(Debug, Eq, PartialEq)]
pub struct CompletedMarker {
    position: usize,
    kind: RawSyntaxKind,
}

impl CompletedMarker {
    /// Returns the completed node kind.
    #[must_use]
    pub const fn kind(&self) -> RawSyntaxKind {
        self.kind
    }

    /// Starts a new marker that will wrap this completed node.
    ///
    /// # Panics
    ///
    /// Panics if this marker's start event is not a start-node event.
    pub fn precede(self, events: &mut Vec<Event>) -> Marker {
        let position = events.len();
        events.push(Event::Tombstone);

        let event = events
            .get_mut(self.position)
            .expect("completed marker position must exist in event stream");
        let Event::Start { forward_parent, .. } = event else {
            panic!("completed marker position must contain a start-node event");
        };
        assert_eq!(
            *forward_parent, NO_FORWARD_PARENT,
            "completed marker must not already have a forward parent"
        );

        *forward_parent = u32::try_from(position - self.position)
            .expect("forward-parent event offset must fit in u32");

        Marker { position }
    }
}

impl Event {
    /// Creates a start-node event with no forward parent.
    #[must_use]
    pub(crate) const fn start_node(kind: RawSyntaxKind) -> Self {
        Self::Start {
            kind,
            forward_parent: NO_FORWARD_PARENT,
        }
    }
}
