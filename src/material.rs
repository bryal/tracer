use nalgebra_glm as glm;
use nalgebra_glm::{vec3, Vec3};
use rand::prelude::*;
use std::f32::consts::{FRAC_1_PI, PI};

#[derive(Clone)]
pub struct Mat {
    pub color: Vec3,
    pub fresnel: Vec3,
    pub shininess: f32,
}

impl Mat {
    pub fn mirror() -> Self {
        Self {
            color: Vec3::repeat(0.0),
            fresnel: vec3(1.0, 1.0, 1.0),
            shininess: 6000.0,
        }
    }

    pub fn diffuse(color: Vec3) -> Self {
        Self {
            color,
            fresnel: Vec3::zeros(),
            shininess: 0.0,
        }
    }
}

// The result of a `*sample_wi` function. A sampled in-direction for a
// out-direction and surface.
pub struct DirSample {
    // wi ⇔ ω_i ⇔ Direction vector towards source of incoming light
    pub wi: Vec3,
    // Probability distribution function. The probability of sampling wi.
    pub pdf: f32,
    // The BRDF for the sampled wi and whichever wo was used to
    // create this sample.
    pub brdf: Vec3,
}

pub fn sample_wi(rng: &mut SmallRng, wo: Vec3, n: Vec3, mat: Mat) -> DirSample {
    Sampler { rng, mat }.dielectric_sample_wi(wo, n)
}

pub fn brdf(wi: Vec3, wo: Vec3, n: Vec3, mat: &Mat) -> Vec3 {
    dielectric_brdf(wi, wo, n, mat)
}

fn dielectric_brdf(wi: Vec3, wo: Vec3, n: Vec3, mat: &Mat) -> Vec3 {
    dielectric_reflection_brdf(wi, wo, n, mat)
        + dielectric_refraction_brdf(wi, wo, n, mat)
}

struct Sampler<'r> {
    rng: &'r mut SmallRng,
    mat: Mat,
}

impl<'r> Sampler<'r> {
    fn rand(&mut self) -> f32 {
        self.rng.gen::<f32>()
    }

    fn dielectric_sample_wi(&mut self, wo: Vec3, n: Vec3) -> DirSample {
        // Russian-roulette sampling of reflection vs refraction.
        //
        // Prefer sampling reflection when the fresnel-parameter `fresnel` (R0)
        // is high.
        let p = 0.5 + glm::comp_min(&self.mat.fresnel) / 2.0;
        if self.rand() < p {
            let mut sample = self.dielectric_reflection_sample_wi(wo, n);
            sample.pdf *= p;
            sample
        } else {
            let mut sample = self.dielectric_refraction_sample_wi(wo, n);
            sample.pdf *= 1.0 - p;
            sample
        }
    }

    // Sample a direction based on the Microfacet brdf
    //
    // # Preconditions
    // `wo` must be on the same side as `n`s plane, i.e. `dot(wo, n) >= 0.0`
    // must be true.
    fn dielectric_reflection_sample_wi(
        &mut self,
        wo: Vec3,
        n: Vec3,
    ) -> DirSample {
        // TODO: Document this math better. TDA362 wasn't very helpful and only
        // said       that it's "out of scope for this tutorial".
        //
        // Importance sample more values where the BRDF-value will be high.
        let phi = 2.0 * PI * self.rand();
        let cos_theta = self.rand().powf(1.0 / (self.mat.shininess + 1.0));
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
        let wh = orthonormal_basis_inverse_transform(
            n,
            vec3(sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta),
        );
        // TODO: Investigate whether `wh` can ever be on the wrong side of
        //       `n`. TDA362 had an early-exit on `dot(wh, n) < 0.0f`.
        let pdf_wh = (self.mat.shininess + 1.0)
            * n.dot(&wh).powf(self.mat.shininess)
            / (2.0 * PI);
        let wi = glm::reflect_vec(&-wo, &wh);
        // TODO: Why exactly can an "invalid" `wi` be generated?
        //
        // If for some reason `wi` is not on the same side as `n`, set
        // probability to 0 to denote that this is an impossible event and
        // the calculated `bdrf` won't make sense. This seems to happen
        // due to bad sampling algorithm. Could maybe be eliminated with a
        // better algorithm.
        let pdf_wi = if wi.dot(&n) >= 0.0 {
            pdf_wh / (4.0 * wo.dot(&wh))
        } else {
            0.0
        };
        DirSample {
            wi,
            pdf: pdf_wi,
            brdf: dielectric_reflection_brdf(wi, wo, n, &self.mat),
        }
    }

    // Sample a direction for the underlying layer
    fn dielectric_refraction_sample_wi(
        &mut self,
        wo: Vec3,
        n: Vec3,
    ) -> DirSample {
        let mut sample = self.diffuse_sample_wi(wo, n);
        sample.brdf =
            attenuate_diffuse_refraction(sample.wi, wo, sample.brdf, &self.mat);
        sample
    }

    fn diffuse_sample_wi(&mut self, wo: Vec3, n: Vec3) -> DirSample {
        let wi = orthonormal_basis_inverse_transform(
            n,
            self.cosine_sample_hemisphere(),
        );
        DirSample {
            // Direction sampled with a cosine distribution
            wi,
            // Cosine probability to match our sampling distribution.
            // Remember, $N ⋅ W = ||N|| ||W|| cos(θ) = 1 * 1 * cos(θ) = cos(θ)$.
            pdf: 0.0f32.max(n.dot(&wi)) * FRAC_1_PI,
            brdf: diffuse_brdf(wi, wo, n, &self.mat),
        }
    }

    fn cosine_sample_hemisphere(&mut self) -> Vec3 {
        let r1 = 2.0 * PI * self.rand();
        let r2 = self.rand();
        let r2s = r2.sqrt();
        vec3(r1.cos() * r2s, r1.sin() * r2s, (1.0 - r2).sqrt())
    }
}

// Torrance-sparrow specular highlight model with approximations.
//
// See [http://www.cse.chalmers.se/edu/year/2018/course/TDA361/Physically-Based%20Shading.pdf]
// and [https://en.wikipedia.org/wiki/Specular_highlight].
fn dielectric_reflection_brdf(wi: Vec3, wo: Vec3, n: Vec3, mat: &Mat) -> Vec3 {
    // `wo` can be on the wrong side of `n` when the geometric normal and
    // shading normal are very different, e.g. due to normal mapping. When
    // this is the case, it doesn't make sense that any light can
    // pass through that route.
    if wo.dot(&n) < 0.0 {
        Vec3::zeros()
    } else {
        let wh = (wo + wi).normalize();
        F(wi, wh, mat.fresnel) * D(wh, n, mat.shininess) * G(wi, wo, wh, n)
            / (4.0 * n.dot(&wo) * n.dot(&wi))
    }
}

// Contribution of the Fresnel factor in the specular reflection
#[allow(non_snake_case)]
fn F(wi: Vec3, wh: Vec3, fresnel: Vec3) -> Vec3 {
    // Schlick's approximation of the contribution of the Fresnel term
    // fresnel ⇔ R0 ⇔ R(θ = 0) ⇔ Reflection coefficient at normal incidence
    fresnel + (Vec3::repeat(1.0) - fresnel) * (1.0 - wh.dot(&wi)).powi(5)
}

// Microfacet distribution.
//
// According to wiki, a physically based model of microfacet
// distribution is the Beckmann distribution, which is good but
// requires more computation than approximate emperical models.
//
// We use a normalized variation of the Phong distribution of the
// Blinn-Phong model $(n ⋅ ω_h)^s$ which is an approximately Gaussian
// distribution for high values of the shininess exponent $s$. Useful
// heuristic with beliavable results, but not a physically based
// model.
//
// To compensate for energy loss at higher shininess, we add a factor
// that normalizes the integral.
#[allow(non_snake_case)]
fn D(wh: Vec3, n: Vec3, shininess: f32) -> f32 {
    (shininess + 2.0) / (2.0 * PI) * n.dot(&wh).powf(shininess)
}

// The geometric attenuation factor, describing selfshadowing due to the
// microfacets
#[allow(non_snake_case)]
fn G(wi: Vec3, wo: Vec3, wh: Vec3, n: Vec3) -> f32 {
    1.0f32.min(
        (2.0 * n.dot(&wh) * n.dot(&wo) / wo.dot(&wh))
            .min(2.0 * n.dot(&wh) * n.dot(&wi) / wo.dot(&wh)),
    )
}

fn dielectric_refraction_brdf(wi: Vec3, wo: Vec3, n: Vec3, mat: &Mat) -> Vec3 {
    return attenuate_diffuse_refraction(
        wi,
        wo,
        diffuse_brdf(wi, wo, n, mat),
        mat,
    );
}

fn attenuate_diffuse_refraction(
    wi: Vec3,
    wo: Vec3,
    brdf: Vec3,
    mat: &Mat,
) -> Vec3 {
    let wh = (wo + wi).normalize();
    (Vec3::repeat(1.0) - F(wi, wh, mat.fresnel)).component_mul(&brdf)
}

fn diffuse_brdf(wi: Vec3, wo: Vec3, n: Vec3, mat: &Mat) -> Vec3 {
    // If `wi` and `wo` are not on the right side of the surface, no light
    // passes through.
    if wo.dot(&n) >= 0.0 && wi.dot(&n) >= 0.0 {
        FRAC_1_PI * mat.color
    } else {
        Vec3::zeros()
    }
}

// To simplify math, we do most (all?) of our vector-sampling around the
// world-up vector, then use this function to transform the vector as if it
// was sampled around the given normal.
//
// E.g. do a hemisphere sample with world-up as center, then transform with
// this to make it as if the hemisphere has n as center.
fn orthonormal_basis_inverse_transform(normal: Vec3, wi: Vec3) -> Vec3 {
    let w_up = if normal.x.abs() > 0.1 {
        vec3(0.0, 1.0, 0.0)
    } else {
        vec3(1.0, 0.0, 0.0)
    };
    let tangent = normal.cross(&w_up).normalize();
    let bitangent = normal.cross(&tangent).normalize();
    tangent * wi.x + bitangent * wi.y + normal * wi.z
}
