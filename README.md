# Remote Audio

## Play a sound

```jsonc
{
  "type": "PlaySound",

  // Id of the sound. Use this to stop or change the sound later on
  "id": "test",

  // Path to the audio file
  "path": "path/to/audio.wav",

  // Play the sound in a loop
  "is_loop": false, // Optional (Default: false)

  // If a sound exists with the same id
  //   true: Stop the exisiting sound and start a new one
  //   false: Don't do anything
  "overwrite": true // Optional (Default: true)
}
```

## Stop a sound

```jsonc
{
  "type": "StopSound",
  "id": "test"
}
```

## Set volume

```jsonc
{
  "type": "SetVolume",
  "id": "test",
  "volume": 0.5
}
```

## Fade to a volume

```jsonc
{
  "type": "FadeToVolume",
  "id": "test",
  "volume": 0.5,
  "time_in_ms": 5000
}
```
