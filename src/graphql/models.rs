/*
amFOSS Daemon: A discord bot for the amFOSS Discord server.
Copyright (C) 2024 amFOSS

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
use serde::Deserialize;
use std::borrow::Cow;

#[derive(Deserialize)]
pub struct Member<'a> {
    id: Option<i32>,
    roll_num: Option<Cow<'a, str>>,
    name: Option<Cow<'a, str>>,
    hostel: &'a str,
    email: &'a str,
    sex: &'a str,
    year: i32,
    mac_addr: &'a str,
    discord_id: &'a str,
    group_id: i32,
}

#[derive(Deserialize)]
struct Data<'a> {
    getMember: Vec<Member<'a>>,
}

#[derive(Deserialize)]
struct Root<'a> {
    data: Data<'a>,
}
