# SPH: Smoothed Particle Hydrodynamics

I've wanted to mess around with SPH for ages, ever since playing
[PixelJunk Shooter](https://en.wikipedia.org/wiki/PixelJunk_Shooter)
back in 2009.

I made a stab at it with Unity in 2012 or thereabout, but it turned into more
of a planetary orbits simulation than a fluid simulation. I never got around 
to figuring out the actual SPH math part of it before I lost interest.

Then I came across
[this excellent video](https://www.youtube.com/watch?v=rSKMYc1CQHE)
by Sebastian Lague, and decided that I just had to revisit it. Naturally, 
since I've been obsessed with Rust for the past couple of years, I decided 
to do it in Rust using the [Bevy game engine](https://bevyengine.org/).