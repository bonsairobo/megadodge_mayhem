use bevy::prelude::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Ray2 {
    pub origin: Vec2,
    pub dir: Vec2,
}

impl Ray2 {
    pub fn new(origin: Vec2, dir: Vec2) -> Self {
        Self { origin, dir }
    }

    pub fn at(&self, t: f32) -> Vec2 {
        self.origin + t * self.dir
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Aabb2 {
    pub min: Vec2,
    pub max: Vec2,
}

impl Aabb2 {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub fn center(&self) -> Vec2 {
        0.5 * (self.min + self.max)
    }

    pub fn shape(&self) -> Vec2 {
        self.max - self.min
    }

    pub fn cast_ray(&self, solid: bool, max_toi: f32, ray: Ray2) -> Option<f32> {
        let mut tmin = 0.0f32;
        let mut tmax = max_toi;

        for i in 0usize..2 {
            if ray.dir[i].abs() < f32::EPSILON {
                if ray.origin[i] < self.min[i] || ray.origin[i] > self.max[i] {
                    return None;
                }
            } else {
                let denom = 1.0 / ray.dir[i];
                let mut inter_with_near_halfspace = (self.min[i] - ray.origin[i]) * denom;
                let mut inter_with_far_halfspace = (self.max[i] - ray.origin[i]) * denom;

                if inter_with_near_halfspace > inter_with_far_halfspace {
                    std::mem::swap(
                        &mut inter_with_near_halfspace,
                        &mut inter_with_far_halfspace,
                    )
                }

                tmin = tmin.max(inter_with_near_halfspace);
                tmax = tmax.min(inter_with_far_halfspace);

                if tmin > tmax {
                    // This covers the case where tmax is negative because tmin is
                    // initialized at zero.
                    return None;
                }
            }
        }

        if tmin.abs() < f32::EPSILON && !solid {
            Some(tmax)
        } else {
            Some(tmin)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Circle {
    pub center: Vec2,
    pub radius: f32,
}

impl Circle {
    pub fn new(center: Vec2, radius: f32) -> Self {
        Self { center, radius }
    }

    pub fn cast_ray(&self, ray: Ray2) -> Option<f32> {
        // Solution found by plugging in parametric equation for line into
        // implicit circle equation, then applying the quadratic formula.
        let p = ray.origin - self.center;
        let a = ray.dir.dot(ray.dir);
        let b = p.dot(ray.dir);
        let c = p.dot(p) - self.radius * self.radius;
        let discrim = b * b - a * c;
        if discrim < 0.0 {
            // No real solution
            None
        } else {
            // Take the smaller of the two solutions.
            let t1 = -(b + discrim.sqrt()) / a;
            let t2 = -(b - discrim.sqrt()) / a;
            let t_first = t1.min(t2);
            // Don't consider negative TOI.
            (t_first >= 0.0).then_some(t_first)
        }
    }
}
