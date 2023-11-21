# Remote Audio

```json
{
  "type": "PlaySound",
  "id": "test",
  "path": "path/to/audio.wav",
  "is_loop": true // Optional
}
```

```json
{
  "type": "StopSound",
  "id": "test"
}
```

```json
{
  "type": "SetVolume",
  "id": "test",
  "volume": 0.5
}
```

```json
{
  "type": "FadeToVolume",
  "id": "test",
  "volume": 0.5,
  "time_in_ms": 5000
}
```
