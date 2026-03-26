use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct BBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl BBox {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn from_xyxy(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self {
            x: x1,
            y: y1,
            w: x2 - x1,
            h: y2 - y1,
        }
    }

    pub fn endx(&self) -> f32 {
        self.x + self.w
    }

    pub fn endy(&self) -> f32 {
        self.y + self.h
    }

    pub fn area(&self) -> f32 {
        self.w * self.h
    }

    pub fn overlaps(&self, other: &BBox) -> bool {
        !(self.endx() < other.x
            || other.endx() < self.x
            || self.endy() < other.y
            || other.endy() < self.y)
    }

    pub fn iou(&self, other: &BBox) -> f32 {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = self.endx().min(other.endx());
        let y2 = self.endy().min(other.endy());

        let intersection_w = (x2 - x1).max(0.0);
        let intersection_h = (y2 - y1).max(0.0);
        let intersection = intersection_w * intersection_h;

        if intersection == 0.0 {
            return 0.0;
        }

        let union = self.area() + other.area() - intersection;
        intersection / union
    }

    /// Intersection area divided by the other box's area (used by NMM).
    pub fn overlap_ratio(&self, other: &BBox) -> f32 {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = self.endx().min(other.endx());
        let y2 = self.endy().min(other.endy());

        let intersection_w = (x2 - x1).max(0.0);
        let intersection_h = (y2 - y1).max(0.0);
        let intersection = intersection_w * intersection_h;

        let other_area = other.area();
        if other_area == 0.0 {
            return 0.0;
        }
        intersection / other_area
    }
}
