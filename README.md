# Bevy Fluid Sim

I've wanted to mess around with fluid simulation (aka SPH, Smoothed Particle
Hydrodynamics) for ages, ever since playing
[PixelJunk Shooter](https://en.wikipedia.org/wiki/PixelJunk_Shooter)
back in 2009.

I made a stab at it with Unity in 2012 or thereabout, but it turned into more
of a planetary orbits simulation than a fluid simulation. I never got around 
to figuring out the math before I lost interest.

## Along came "Coding Adventure: Simulating Fluids"

Recently I came across
[this excellent video](https://www.youtube.com/watch?v=rSKMYc1CQHE)
by Sebastian Lague, and it rekindled my interest. Naturally, 
since I've been obsessed with Rust for the past couple of years, I decided 
to give it a go in Rust using the [Bevy game engine](https://bevyengine.org/).

## Early Results

After wrestling with it for a week or two, I've finally achieved something that
looks... fluid adjacent.

![fluid adjacent](./Screenshot%202025-03-02%20at%206.59.13%E2%80%AFPM.png).

## Running

To speed up compiling (world record understatement), I use the `dynamic_linking`
bevy feature. Therefore, running the app requires the following:

### Mac

```
DYLD_FALLBACK_LIBRARY_PATH=$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/lib ./target/release/bevy-fluid-sim
```

### Windows

```
PATH=%USERPROFILE%\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\;.\target\debug\deps\;.\target\release\deps
.\target\release\bevy-fluid-sim.exe
```
