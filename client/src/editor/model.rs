//! What the editor edits, without anything shown (docs/editor.md): a circuit as a list of pieces,
//! each a straight or a turn carrying everything the player sets on it, and how that list becomes
//! the track file the race builds from.
//!
//! A piece holds its own width and the height the road reaches at its end, so the player shapes
//! the road piece by piece; the track file wants them as changes of width and heights along the
//! lap ([`Draft::description`] places them there). The road need not end where it began: the
//! editor closes it with the shortest way back to the line (`editing::closing`), the width coming
//! back to the first piece's and the road down to the line's height on the way.

use serde::{Deserialize, Serialize};
use star_racer_sim::track::editing;
use star_racer_sim::track::{
    Elevation, MAX_WIDTH, MIN_WIDTH, Segment, TrackDescription, Widening,
};

/// What a piece of road is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Straight,
    /// A turn, to the left for a positive angle.
    Turn,
}

/// One piece of the road, laid after the one before it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Piece {
    pub kind: Kind,
    /// A straight's length, in meters.
    pub length: f32,
    /// A turn's angle in degrees, positive to the left, and its radius in meters.
    pub angle: f32,
    pub radius: f32,
    /// How steeply a turn is banked toward its outside, in degrees.
    pub banking: f32,
    /// The road's width along the piece, from wall to wall, in meters.
    pub width: f32,
    /// The height of the road where the piece ends, in meters.
    pub height: f32,
}

impl Piece {
    pub fn straight(length: f32, width: f32, height: f32) -> Self {
        Self {
            kind: Kind::Straight,
            length,
            angle: 90.0,
            radius: 80.0,
            banking: 0.0,
            width,
            height,
        }
    }

    pub fn turn(angle: f32, radius: f32, width: f32, height: f32) -> Self {
        Self {
            kind: Kind::Turn,
            length: 150.0,
            angle,
            radius,
            banking: 0.0,
            width,
            height,
        }
    }

    /// The segment of the track file the piece is. A turn eases in and out of its curve over a
    /// transition of a fifth of its length, at most 40 m, so steering into it is progressive, and
    /// a banked turn has the transition it needs to rise into its banking.
    pub fn segment(&self) -> Segment {
        match self.kind {
            Kind::Straight => Segment::Straight {
                length: self.length,
            },
            Kind::Turn => {
                let arc = self.angle.abs().to_radians() * self.radius;
                Segment::Turn {
                    angle: self.angle,
                    radius: self.radius,
                    transition: (arc * 0.2).min(40.0),
                    banking: self.banking,
                }
            }
        }
    }

    /// The number the player sets on it, by name, as the panel steps them.
    pub fn get(&self, field: Field) -> f32 {
        match field {
            Field::Length => self.length,
            Field::Angle => self.angle,
            Field::Radius => self.radius,
            Field::Banking => self.banking,
            Field::Width => self.width,
            Field::Height => self.height,
        }
    }

    /// Steps `field` by `steps` of its step, kept within its bounds. A turn stepped through zero
    /// turns the other way rather than becoming a straight.
    pub fn step(&mut self, field: Field, steps: f32) {
        let (step, low, high) = field.bounds();
        let value = self.get(field) + step * steps;
        let value = match field {
            Field::Angle => {
                let flipped = if value.abs() < 5.0 {
                    // Through zero: the other way round, at the same small angle.
                    -self.angle.signum() * 5.0
                } else {
                    value
                };
                flipped.clamp(low, high)
            }
            _ => value.clamp(low, high),
        };
        match field {
            Field::Length => self.length = value,
            Field::Angle => self.angle = value,
            Field::Radius => self.radius = value,
            Field::Banking => self.banking = value,
            Field::Width => self.width = value,
            Field::Height => self.height = value,
        }
    }
}

/// The numbers a piece has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Length,
    Angle,
    Radius,
    Banking,
    Width,
    Height,
}

impl Field {
    /// The fields shown for a piece of `kind`, in order.
    pub fn of(kind: Kind) -> &'static [Field] {
        match kind {
            Kind::Straight => &[Field::Length, Field::Width, Field::Height],
            Kind::Turn => &[
                Field::Angle,
                Field::Radius,
                Field::Banking,
                Field::Width,
                Field::Height,
            ],
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Field::Length => "LENGTH",
            Field::Angle => "ANGLE",
            Field::Radius => "RADIUS",
            Field::Banking => "BANKING",
            Field::Width => "WIDTH",
            Field::Height => "HEIGHT",
        }
    }

    pub fn unit(self) -> &'static str {
        match self {
            Field::Angle | Field::Banking => "°",
            _ => " m",
        }
    }

    /// One step of the field, and the lowest and highest it goes.
    pub fn bounds(self) -> (f32, f32, f32) {
        match self {
            Field::Length => (10.0, 10.0, 2000.0),
            Field::Angle => (5.0, -350.0, 350.0),
            Field::Radius => (5.0, 20.0, 800.0),
            Field::Banking => (2.0, 0.0, 50.0),
            Field::Width => (2.0, MIN_WIDTH, MAX_WIDTH),
            Field::Height => (2.0, -100.0, 200.0),
        }
    }
}

/// A circuit being laid: its name, the road as the player laid it, and the radius of the turns
/// that close it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Draft {
    pub name: String,
    pub pieces: Vec<Piece>,
    pub closing_radius: f32,
}

/// What a piece may be added as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum New {
    Straight,
    Left,
    Right,
}

/// A circuit once closed and turned into a track file: the description, and how many of its
/// segments the editor added to close it.
pub struct Closed {
    pub description: TrackDescription,
    pub closing: usize,
}

impl Draft {
    /// A first circuit to start from: a straight out of the line, a long left, a straight back.
    pub fn starter() -> Self {
        Self {
            name: "My Circuit".into(),
            pieces: vec![
                Piece::straight(300.0, 30.0, 0.0),
                Piece::turn(180.0, 120.0, 30.0, 0.0),
                Piece::straight(200.0, 30.0, 0.0),
            ],
            closing_radius: 120.0,
        }
    }

    /// Adds a piece after `after` (at the end without one), shaped like its neighbor: as wide and
    /// as high, a turn of 90 degrees. Answers where it went.
    pub fn add(&mut self, new: New, after: Option<usize>) -> usize {
        let at = after.map_or(self.pieces.len(), |index| (index + 1).min(self.pieces.len()));
        let neighbor = at.checked_sub(1).and_then(|index| self.pieces.get(index));
        let (width, height) = neighbor.map_or((30.0, 0.0), |piece| (piece.width, piece.height));
        let piece = match new {
            New::Straight => Piece::straight(200.0, width, height),
            New::Left => Piece::turn(90.0, 100.0, width, height),
            New::Right => Piece::turn(-90.0, 100.0, width, height),
        };
        self.pieces.insert(at, piece);
        at
    }

    /// Meters along the lap where each piece starts, and where the last one ends.
    pub fn starts(&self) -> Vec<f32> {
        let mut at = 0.0;
        let mut out = Vec::with_capacity(self.pieces.len() + 1);
        out.push(0.0);
        for piece in &self.pieces {
            at += piece.segment().length();
            out.push(at);
        }
        out
    }

    /// The road as laid, not closed: the pieces' segments alone, flat and as wide as the first
    /// piece, which the editor traces when it cannot be closed. Nothing without a piece.
    pub fn open(&self) -> Option<TrackDescription> {
        let first = self.pieces.first()?;
        Some(TrackDescription {
            name: self.name.trim().to_owned(),
            width: first.width,
            segments: self.pieces.iter().map(Piece::segment).collect(),
            elevation: Vec::new(),
            widths: Vec::new(),
            narrows: Vec::new(),
            scenery: Vec::new(),
            pads: Vec::new(),
            gaps: Vec::new(),
            checkpoints: Vec::new(),
            breaches: Vec::new(),
        })
    }

    /// The circuit as the track file it races from: the pieces' segments and the way back to the
    /// line, the widths changing where a piece is not as wide as the one before it, and a height
    /// at the end of every piece, the line at the first piece's start height (0). Or why it cannot
    /// be closed.
    pub fn description(&self) -> Result<Closed, String> {
        let open = self.open().ok_or("lay at least one piece of road")?;
        let first = self.pieces[0];
        let closing = editing::closing(&open, self.closing_radius).map_err(str::to_owned)?;
        let starts = self.starts();
        let laid = *starts.last().expect("there is a start");
        let length = laid + closing.iter().map(Segment::length).sum::<f32>();

        let mut description = open;
        description.segments.extend(closing.iter().copied());
        // A change of width where a piece is not as wide as the one before it, and back to the
        // first piece's width on the way home.
        let mut previous = first.width;
        description.widths.push(Widening {
            at: 0.0,
            width: first.width,
            blend: 0.0,
        });
        for (piece, &at) in self.pieces.iter().zip(&starts).skip(1) {
            if piece.width != previous {
                description.widths.push(Widening {
                    at,
                    width: piece.width,
                    blend: 30.0,
                });
                previous = piece.width;
            }
        }
        if previous != first.width && laid < length - 1.0 {
            description.widths.push(Widening {
                at: laid,
                width: first.width,
                blend: (length - laid).min(30.0),
            });
        }
        // The line at height 0, and the road through each piece's end height; the way home comes
        // back down to the line.
        description.elevation.push(Elevation {
            at: 0.0,
            height: 0.0,
        });
        for (piece, &end) in self.pieces.iter().zip(&starts[1..]) {
            if end < length - 1.0 {
                description.elevation.push(Elevation {
                    at: end,
                    height: piece.height,
                });
            }
        }
        if description.elevation.iter().all(|key| key.height == 0.0) {
            description.elevation.clear();
        }
        Ok(Closed {
            description,
            closing: closing.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use star_racer_sim::track::{MIN_CLEARANCE, Track, TrackError};

    /// The first circuit builds, closed by the editor.
    #[test]
    fn the_starter_builds() {
        let closed = Draft::starter().description().expect("it closes");
        assert!(closed.closing > 0);
        Track::build(&closed.description).expect("it builds");
    }

    /// Pieces added go after the one selected, shaped like it; stepping keeps within bounds, and a
    /// turn stepped through zero turns the other way.
    #[test]
    fn pieces_are_added_and_stepped() {
        let mut draft = Draft::starter();
        draft.pieces[0].width = 40.0;
        let at = draft.add(New::Left, Some(0));
        assert_eq!(at, 1);
        assert_eq!(draft.pieces[1].kind, Kind::Turn);
        assert_eq!(draft.pieces[1].width, 40.0);
        assert_eq!(draft.add(New::Straight, None), draft.pieces.len() - 1);

        let mut turn = Piece::turn(10.0, 100.0, 30.0, 0.0);
        turn.step(Field::Angle, -1.0);
        assert_eq!(turn.angle, 5.0);
        turn.step(Field::Angle, -1.0);
        assert_eq!(turn.angle, -5.0, "through zero, the other way");
        turn.step(Field::Width, 1000.0);
        assert_eq!(turn.width, MAX_WIDTH);
    }

    /// A piece's width and height go where it is on the lap: the road as wide along it, and as
    /// high at its end.
    #[test]
    fn widths_and_heights_go_where_their_pieces_are() {
        let mut draft = Draft::starter();
        draft.pieces[2].width = 50.0;
        draft.pieces[1].height = 12.0;
        let closed = draft.description().unwrap();
        let track = Track::build(&closed.description).expect("it builds");
        let starts = draft.starts();
        let middle_of_last = (starts[2] + starts[3]) / 2.0;
        assert!((2.0 * track.half_width_at(middle_of_last) - 50.0).abs() < 0.5);
        assert!((2.0 * track.half_width_at(starts[1] / 2.0) - 30.0).abs() < 0.5);
        assert!((track.point_at(starts[2]).height - 12.0).abs() < 0.1);
        assert!(track.point_at(0.0).height.abs() < 0.1);
    }

    /// Laid so it crosses itself, the circuit builds with a bridge high enough, and says why not
    /// at the same level.
    #[test]
    fn a_crossing_needs_a_bridge() {
        let mut draft = Draft {
            name: "Over".into(),
            pieces: vec![
                Piece::straight(300.0, 24.0, 0.0),
                Piece::turn(225.0, 60.0, 24.0, 0.0),
                Piece::straight(400.0, 24.0, 0.0),
            ],
            closing_radius: 60.0,
        };
        let flat = draft.description().unwrap();
        assert!(matches!(
            Track::build(&flat.description),
            Err(TrackError::Overlaps { .. })
        ));
        // High enough over both roads it crosses: the straight out of the line, and the way home,
        // which climbs a little toward it on its way down.
        draft.pieces[1].height = MIN_CLEARANCE + 10.0;
        draft.pieces[2].height = MIN_CLEARANCE + 10.0;
        let raised = draft.description().unwrap();
        if let Err(error) = Track::build(&raised.description) {
            panic!("raised, it did not build: {error}");
        }
    }
}
