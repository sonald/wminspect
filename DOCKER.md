# Docker Usage for wminspect

## Building the Docker Image

To build the Docker image locally:

```bash
docker build -t wminspect .
```

## Running with Docker

### Basic Usage

```bash
# Run wminspect with X11 forwarding
docker run --rm -it \
  -v /tmp/.X11-unix:/tmp/.X11-unix:rw \
  -e DISPLAY=$DISPLAY \
  wminspect --help
```

### Using Docker Compose

For easier management with X11 support:

```bash
# Start the services
docker-compose up -d

# Run wminspect
docker-compose exec wminspect wminspect --help

# Stop the services
docker-compose down
```

## Environment Variables

- `DISPLAY`: Required for X11 forwarding
- `RUST_LOG`: Set logging level (debug, info, warn, error)

## Volumes

- `/tmp/.X11-unix`: X11 socket for GUI applications
- `/home/wminspect`: Working directory inside container

## Network

The container runs in host network mode to access the X11 server.

## Security Notes

- The container runs as a non-root user `wminspect`
- X11 forwarding requires appropriate permissions on the host
- Consider using xhost or other X11 security measures as needed

## Troubleshooting

### X11 Permission Issues

If you encounter X11 permission errors:

```bash
# Allow X11 connections (use with caution)
xhost +local:docker

# Or more securely, allow specific user
xhost +si:localuser:$(whoami)
```

### Display Issues

Ensure DISPLAY is set correctly:

```bash
echo $DISPLAY
```

If empty, try:

```bash
export DISPLAY=:0.0
```
