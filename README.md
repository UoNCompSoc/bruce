# Bruce Bot

This Discord bot is designed to allow easy member management in a society discord.

## Setup

You need 2 things for this bot to work:

- Access to the members list for your society, found on a url like `https://student-dashboard.sums.su/groups/336/members`
- A Discord bot and token for it

### Cookie for SUMS

1. Install a cookie viewer extension/addon for your browser (I used [Cookie Quick Manager](https://github.com/ysard/cookie-quick-manager))
2. Login to your student dashboard
3. Using your cookie tool, find the cookie with url `student-dashboard.sums.su` and name `su_session`.
4. Save the value of that cookie, it should look something like this: `dvpsnk67tme2eal2qu44627o4p20iviv`
5. Note that the IP that you obtain the cookie from must be the same one that the bot will use.

### Discord bot

1. Create a new application on the [Discord developer portal](https://discord.com/developers/applications)
2. Add a Bot to your application
3. Press `Reset Token` and save the token that gets generated
4. Disable the `PUBLIC BOT` option
5. Under `Privileged Gateway Intents`, enable `Message Content Intent`
6. In your application settings, go to `OAuth2 > URL Generator` and create a link:
    1. Under `SCOPES`, check the `bot` and `applications.commands` boxes
    2. Under `BOT PERMISSIONS`, check `Manage Roles`, `Manage Nicknames`, `Read Messages/View Channels` and `Send Messages`
    3. Open the generated URL and add the bot to your desired server
    4. In your server, you can allow the bot into specific channels etc via its role.

## Installation

1. Create a folder on a linux machine with docker installed
2. Download the [docker-compose.yml](https://github.com/UoNCompSoc/bruce/blob/main/docker-compose.yml) and place it in that folder
3. Download the [example.env](https://github.com/UoNCompSoc/bruce/blob/main/example.env), rename it to `.env` and place it in the same folder
4. Fill out the `.env` file with the details we collected earlier, there's a breakdown of each variable below. The mandatory ones are: `DISCORD_TOKEN`, `MEMBERS_URL` and `INITIAL_SUMS_COOKIE_VALUE`
5. Start the container with `docker-compose up -d` and check the logs with `docker-compose logs`
6. In your Discord server, send a message (where the bot can see it): `bruce!setup_commands`, this will give Discord the list of slash commands the bot has.
7. Now you can use the slash commands by typing a `/` and picking the one you want.

### Variables

| Key                       | Optional                                                            | Default   | Example                                                                 | Description                                                          |
|---------------------------|---------------------------------------------------------------------|-----------|-------------------------------------------------------------------------|----------------------------------------------------------------------|
| MEMBERS_URL               | False                                                               | N/A       | https://student-dashboard.sums.su/groups/336/members                    | This page should contain the list of members of your society         |
| DISCORD_TOKEN             | False                                                               | N/A       | GHk1MzU6MDkwODk3MTA4OTad.GmurJI.1DH4qad-Q635rkYvaRDfPRl1u5HM--8kKUH_aZ  | This is the token we got from the Discord developers portal above    |
| INITIAL_SUMS_COOKIE_VALUE | True (but you'll need it for the first run or if the token expires) | N/A       | dlesnk67tme2eal2qu44627o4p69iviq                                        | This is the value we got from the cookie tool                        |
| MEMBER_ROLE_NAME          | True                                                                | Member    | N/A                                                                     | This is the role that the bot will give your members                 |
| PRIVILEGED_ROLE_NAME      | True                                                                | Committee | N/A                                                                     | This is the role of people that can run the bots management commands |
| MEMBERSHIP_PURCHASE_URL   | True                                                                | N/A       | https://su.nottingham.ac.uk/shop/product/31-computer-science-membership | This is a link that your members can go to to purchase a membership  |

## Bot Usage

Bruce has 3 main commands:

### /register

Register allows any user to provide their student id to verify that they are a member of the society. If the check passes, Bruce will give them your defined member role and also set their nickname to their real name.

### /unregister

Unregister allows privileged users (usually committee) to unregister a specific discord user in the event something goes awry. For example, a user may /register with a student id other than their own.  
Unregistering a user should be done via Bruce otherwise Bruce will still think that the user is registered. Removing the user's role is not enough.

### /prune

Prune allows privileged users (usually committee) to bulk unregister users whose memberships have expired. This would be a scheduled task however since memberships can be bought at any time of the year, I decided to leave it up to the society to decide when to prune.
