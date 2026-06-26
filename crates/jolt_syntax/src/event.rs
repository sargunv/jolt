use crate::{Diagnostic, RawSyntaxKind};

/// A parser event consumed by the shared green tree sink.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    /// Starts a syntax node with the supplied raw kind.
    StartNode {
        /// The node kind.
        kind: RawSyntaxKind,
        /// Offset to a later start-node event that should wrap this node.
        forward_parent: Option<usize>,
    },
    /// Consumes the next token from the token source.
    Token,
    /// Finishes the current syntax node.
    FinishNode,
    /// Records a parser diagnostic without changing the tree shape.
    Error(Diagnostic),
    /// A placeholder event reserved by parser marker APIs.
    Tombstone,
}

/// A placeholder for a syntax node whose kind is not known yet.
#[derive(Clone, Debug, Eq, PartialEq)]
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
        events.push(Event::FinishNode);

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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletedMarker {
    position: usize,
    kind: RawSyntaxKind,
}

impl CompletedMarker {
    /// Returns the start event position for this completed marker.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }

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
        let Event::StartNode { forward_parent, .. } = event else {
            panic!("completed marker position must contain a start-node event");
        };
        assert!(
            forward_parent.is_none(),
            "completed marker must not already have a forward parent"
        );

        *forward_parent = Some(position - self.position);

        Marker { position }
    }
}

impl Event {
    /// Creates a start-node event with no forward parent.
    #[must_use]
    pub const fn start_node(kind: RawSyntaxKind) -> Self {
        Self::StartNode {
            kind,
            forward_parent: None,
        }
    }
}
