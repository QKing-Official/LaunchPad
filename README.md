# LaunchPad Daemon

This is the LaunchPad daemon. This is a simple, fast and highly optimised daemon for simple docker container management.
This project uses bollard to interact with docker and Postgresql as database.
This Daemon is kept simple and should be used on projects that do not require a giant daemon like Pterodactyl Wings.
Even though this daemon is small, I've added a couple nice features.

## API

You can interact with the LaunchPad daemon in one way currently. The HTTP API. It will run on the port set in the .env. More about that later. You can devide the API in 2 sections (and 2 routes that do not need authentication), user interaction and admin interaction. Admin interaction happens with the global API key set in the .env file (more about that later) and can be used on all routes in the daemon. User tokens are generated per app and can be used on all in container management routes like shutting the container down and checking the resource usage. 

### Unauthenticated routes

For these routes you do not need to authenticate to the daemon. You can freely visit them.

`/` This is the main route that is served on the root url of the bind adress. This route usually just says `deamon alive` if the daemon is reachable. In the demo (where many features are restricted) is it used to serve the testing ui with buttons for the different accesible api routes.

`/health` This route is similar to `/` and will display `ok` when the daemon is reachable

### Admin-only endpoints

For these routes you have to use the API_KEY set in the .env (more about that later).
Those routes are usually very powerful, hence why only the admin can use them.

`/servers` This route is used to display server information

`/apps` This route is used to create, delete and manage services. `/apps/{id}` is a subroute of it to perform actions on the server like deletion.

### User token endpoints

User token endpoints are endpoints that can be used with the app-key that can be created for each individual server. The global API key will also work on every endpoint here.

`/apps/{id}/power` Controls the power state of an app (start, stop, restart).

`/apps/{id}/exec` Executes a command inside a running app's container.

`/apps/{id}/logs` Retrieves the logs of an app.

`/apps/{id}/stats` Returns runtime monitoring stats for an app.

`/apps/{id}/ports` Lists and manages port mappings exposed by an app. `/apps/{id}/ports/{mapping_id}` targets a specific mapping for deletion.

`/apps/{id}/files` Manages the filesystem of an app. List, read, upload, and delete files.

`/apps/{id}/network` Inspects and controls network connections for an app. `/apps/{id}/network/connect` and `/network/disconnect` link or unlink apps from a shared network.

`/apps/{id}/webhooks` Lists and manages webhooks attached to an app. `/apps/{id}/webhooks/{wh_id}` targets a specific webhook for deletion.

`/apps/{id}/tokens` Lists and manages per-app user tokens. `/apps/{id}/tokens/{tok_id}` targets a specific token for deletion.

## How to use it?

You can check out the demo on https://launchpad.qking.me but many features are locked for safety of my own systems. I advise you to run it yourself. For the full endpoint and API documentation, see the README in the daemon folder

### Step 1 dependencies

You will need a couple dependencies to run the daemon from the binary provided in the release file.
If you want to compile and run it yourself you only need rust and cargo extra (see https://rust-lang.org/tools/install/) and you can do cargo run inside the directory you cloned.

Postgresql:
I have used postgresql since it was easy to use. You install it using the following guide: https://www.postgresql.org/download/linux/ubuntu/ 


Docker:
Our daemon is build on top of docker, so you will have to install that as well. I recommend following the official documentation for it. https://docs.docker.com/engine/install/ubuntu/.

### Step 2 Setup

Now that we have installed all dependecies, we can setup the files required to run the daemon now.
Download the binary from the github release page and put it inside a directory of choice.

We will need to create the database fist.
Run `psql -U postgres` to enter the postgresql console.
We will have to create a database there. You can do it using the following command `CREATE DATABASE daemon;`.
This creates a database named daemon, you can put any name you want there but you will have to change something later.
To exit the postgresql shell run `\q`.

Now that we have setup the database we need to setup the directories for volumes that can be mounted to the containers. Do that by running the following commands.
```bash
sudo mkdir -p /srv/Launchpad
sudo chown build:build /srv/Launchpad
```
This ensures the volumes work later.

There is now one thing left to do. Setup the .env file.
The .env file is the file where all secrets and configuration is stored securely.
Please make a .env file in the directory the binary is in and paste the following thing inside.
```env
PORT=8000
# Generate a strong key: openssl rand -hex 32
# API_KEY=<set this before running. Empty means not working>
DATABASE_URL=postgres://daemon:daemonpass@localhost:5432/daemon
RUST_LOG=daemon=debug,info
```
You can change the log level of rust by adding or removing arguments
PORT is the variable of the port it runs on, change it to whatever you want.
To generate the required api key, uncomment the line and run the following command.
`openssl rand -hex 32` Paste the output in after the = line.
It will be something like `API_KEY=2167d744454ea36c8bffebd765bd1c8f63df520509e36a7a8f1b7541bc9f8863`.
Never share your API key with someone.

By default the daemon binds to 127.0.0.1. If you do not want it add this line:
```
BIND_ADDR=0.0.0.0
```
You can replace 0.0.0.0 with the bind adress of choice.

If you have changed the database name in the first step. Replace /daemon in the datapass url with the name of the current database. Also replace daemon and daemonpass with the real password and username of your database user.

### Step 3 Running

Finally, you can run the daemon.
To do that just run the binary `./daemon` and watch the output.

## My journey

I wanted to make a host where people could get a free container to host their HackClub project on (I'm not affiliated with HackClub).
Since hackclub is coding related I just cant use an existing panel in my mindset.
That is why I am writing this daemon.
This daemon is written in Rust and can compile to single binary.
This uses postgresql and is pretty optimised I must say.
I am also submitting this into the lockin sidequest of HackClub.

This is the solution, a custom daemon with all features I need.
Some are weird I must say. For example webhooks and the network isolation.
I wanted it to be secure so I got to work.

If you are reading this it is probably not done yet, but it works.
I advise you to not use this in production at the time I write this.
This readme will be updated and there will be stated otherwise when it is fully done.

If there are any issues, just create an issue here!

I am going to work on Lockin week 3, cya!