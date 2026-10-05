# Demon den

The invasion entrance is a single rocky cavern spanning the road's enemy end. The former green portal, ruined gate posts and flat purple stains have been removed.

The cavern uses a dark recessed mouth, fifteen rock arch sections, six fangs, violet fissures, a dim interior light and twelve translucent mist wisps. Its art faces the production orthographic camera; the camera and real enemy spawn coordinates remain unchanged. The cavern does not own collision or enemy AI.

Newly spawned enemies trigger a short emergence pulse in their lane. Pool reactivation also adds the Enemy component and triggers this feedback. The pulse lasts 0.75 seconds, is limited to 32 concurrent effects, freezes while paused and clears on a run reset. Persistent cavern geometry survives chapter transitions.

Native capture scenario `demon-cave` checks the quiet cavern, real wave spawning and observed emergence feedback using the production camera at desktop and mobile viewport sizes.
