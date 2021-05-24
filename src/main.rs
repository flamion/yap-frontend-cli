use cursive;
use cursive::align::HAlign;
use cursive::Cursive;
use cursive::event::Key;
use cursive::theme::ColorStyle;
use cursive::view::{Nameable, Resizable, SizeConstraint};
use cursive::views::{Dialog, EditView, LinearLayout, TextView, Checkbox, PaddedView, SelectView,
                     ScrollView, ResizedView, Layer, StackView, Panel, TextArea};
use regex::Regex;
use std::ops::Not;
use reqwest::blocking;
use reqwest::StatusCode;
use std::io::{Write, ErrorKind};
use std::fs;
use std::vec;
use std::fs::{File, OpenOptions};
use serde_json;
use std::path::{Path, PathBuf};
use serde::Serialize;
use serde::Deserialize;
use xdg;
use chrono;
use cursive_calendar_view::{CalendarView, EnglishLocale, ViewMode};
use chrono::{Date, Local, Timelike};
//use std::thread;
//use std::sync::mpsc;




//TODO rewrite login so it takes email and password as arguments
//TODO write a function which takes enums and matches them to strings as the cursive-name replacement

//TOKEN_FILE name
static TOKEN_FILE: &'static str = "token.json";
static BASE_URL: &'static str = "https://backend.yap.dragoncave.dev";

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
struct BoardAPI {
    boardID: i64,
    name: String,
    createDate: i64,
    creatorID: i64, //UserID
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
struct UserAPI {
    userID: i64,
    username: String,
    createDate: i64,
    lastLogin: i64,
    emailAddress: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
struct EntryAPI {
    entryID: i64,
    creatorID: i64,
    createDate: i64,
    dueDate: i64,
    title: String,
    description: String,
}

#[derive(Debug, Clone)]
struct Board {
    board_id: i64,
    name: String,
    create_date: chrono::DateTime<chrono::offset::Local>,
    creator_id: i64,
}

#[derive(Debug, Clone)]
struct User {
    user_id: i64,
    name: String,
    create_date: chrono::DateTime<chrono::offset::Local>,
    last_login: chrono::DateTime<chrono::offset::Local>,
    email_address: String,
}

#[derive(Debug, Clone)]
struct Entry {
    entry_id: i64,
    creator_id: i64,
    create_date: chrono::DateTime<chrono::offset::Local>,
    due_date: chrono::DateTime<chrono::offset::Local>,
    title: String,
    description: String,
}

enum EntryItem {
    Entry(Entry),
    Add(i64),
}

enum BoardItem {
    Board(Board),
    Add,
}

enum BackendError {
    Incomplete, //400
    Deleted, //204
    TokenInvalid, //401
    NoAccess, //403
}

struct Tab {
    indicator: &'static str,
    layer: &'static str,
}

//Used to "emulate" enums, as Cursive only supports Strings as names
static TABS: [Tab;2] = [
    Tab {
        indicator: "BOARDS_TAB_INDICATOR",
        layer: "BOARDS_TAB_LAYER",
    },
    Tab {
        indicator: "ACCOUNT_TAB_INDICATOR",
        layer: "ACCOUNT_TAB_LAYER",
    }
];

struct GlobalData {
    http_client: blocking::Client,
    token: Option<String>,
    config_home: xdg::BaseDirectories,
}

#[derive(Serialize, Deserialize)]
struct TokenFile {
    user_mail: String,
    token: String,
}

enum RegisterInvalid {
    InvalidUsername,
    InvalidEmail,
    InvalidPassword,
}

enum TokenLoadError {
    TokenExpired,
    FileNotFound,
    FileNotReadable,
}



fn set_entry_nav_callback(siv: &mut Cursive, status: bool) {
    if status {
        siv.add_global_callback(
            Key::Esc,
            |s| {
                switch_stack(
                    s,
                    "BOARD_STACK",
                    "BOARD_LAYER"
                );
                clear_entry_view(s);
            }
        );
    } else {
        siv.clear_global_callbacks(Key::Esc);
    }
}

fn set_tab_nav(siv: &mut Cursive, state: bool) {
    if state {
        siv.add_global_callback(Key::Left, |siv| select_tab(siv, &TABS[0]));
        siv.add_global_callback(Key::Right, |siv| select_tab(siv, &TABS[1]));
    } else {
        siv.clear_global_callbacks(Key::Left);
        siv.clear_global_callbacks(Key::Right);
    }
}

fn set_callbacks(siv: &mut Cursive, state: bool) {
    set_entry_nav_callback(siv, state);
    set_tab_nav(siv, state)
}


fn select_tab(siv: &mut Cursive, tab_name: &Tab) {
    for tab in &TABS {
        let mut tab_indicator = siv.find_name::<Layer<TextView>>(tab.indicator)
            .expect("tab indicator not found");

        if tab.indicator.eq(tab_name.indicator) {
            tab_indicator.set_color(ColorStyle::highlight());
            tab_indicator.get_inner_mut().set_style(ColorStyle::highlight());
        } else {
            tab_indicator.set_color(ColorStyle::primary());
            tab_indicator.get_inner_mut().set_style(ColorStyle::primary());
        }
    }

    let mut tab_layers = siv.find_name::<StackView>("TAB_LAYERS")
        .expect("tab layers not found");

    let tab_layer = tab_layers.find_layer_from_name(tab_name.layer)
        .expect("tab layer not found");

    tab_layers.move_to_front(tab_layer);

    siv.clear_global_callbacks(Key::Esc);

    if tab_name.layer == TABS[0].layer {
        set_entry_nav_callback(siv, true);
    }
}

/*fn reload_all(siv: &mut Cursive) {

}*/

fn load_boards_to_view(siv: &mut Cursive) {
    match get_board_ids(siv) {
        Ok(board_ids) => {
            for board_id in board_ids {
                match get_board_from_id(siv, board_id) {
                    Ok(board_obj) => {
                        load_to_board_view(siv, board_obj);
                    },
                    Err(error) => error_handler(siv, error),
                }
            }
        },
        Err(error) => error_handler(siv, error),
    }
}

fn load_entries_to_view(siv: &mut Cursive, board_id: i64) {
    match get_board_entry_ids(siv, board_id) {
        Ok(entry_ids) => {
            for entry_id in entry_ids {
                match get_entry_from_id(siv, entry_id) {
                    Ok(entry_obj) => {
                        load_to_entry_view(siv, entry_obj);
                    },
                    Err(error) => error_handler(siv, error),
                }
            }
        },
        Err(error) => error_handler(siv, error),
    }

    siv.find_name::<SelectView<EntryItem>>("ENTRY_SELECTION")
        .expect("view: 'ENTRY_SELECTION' not found")
        .add_item("<add new entry>", EntryItem::Add(board_id));
}

fn create_board(siv: &mut Cursive, name: &str) -> Result<i64, BackendError> {
    let token = &siv.user_data::<GlobalData>().expect("no token")
        .token
        .clone()
        .expect("clone failed");

    match siv.user_data::<GlobalData>()
        .expect("no user data set")
        .http_client
        .post(format!("{}/boards", BASE_URL))
        .header("token", token)
        .json::<BoardAPI>(&BoardAPI {
            boardID: 0,
            name: name.to_string(),
            createDate: 0,
            creatorID: 0
        })
        .send() {

        Ok(response) => if response.status().is_success() {
            return Ok(response.json::<i64>()
                .expect("didn't recieve an i64"));
        } else {
            return Err(error_converter(response.status()));
        },
        Err(error) => panic!("{}", verbose_panic(error)),
    };
}

fn on_submit_board(siv: &mut Cursive, item: &BoardItem) {
    match item {
        BoardItem::Board(board) => {
            load_entries_to_view(siv, board.board_id);
            switch_stack(siv, "BOARD_STACK", "ENTRY_LAYER");
        },
        BoardItem::Add => {
            siv.add_layer(
                Dialog::new()
                    .title("Create new board")
                    .content(
                        LinearLayout::vertical()
                            .child(
                                TextView::new("\nHow should it be called?\n")
                                    .h_align(HAlign::Center)
                                    .fixed_height(3)
                            )
                            .child(
                                EditView::new()
                                    .fixed_width(24)
                                    .with_name("BOARD_CREATE_NAME")
                            )
                    )
                    .button("cancel", |s| {
                        s.pop_layer();
                        set_tab_nav(s, true);
                    }
                    )
                    .button(
                        "create", |s| {
                            let name = &s.find_name::<ResizedView<EditView>>("BOARD_CREATE_NAME")
                                .expect("view: 'BOARD_CREATE_NAME' not found")
                                .get_inner_mut()
                                .get_content()
                                .clone();

                            match create_board(s, name) {
                                Ok(board_id) => {
                                    match get_board_from_id(s, board_id) {
                                        Ok(board) => load_to_board_view(s, board),
                                        Err(error) => error_handler(s, error),
                                    }
                                },
                                Err(error) => error_handler(s, error),
                            };
                            s.pop_layer();
                            set_tab_nav(s, true);
                        },
                    )
            );
            set_tab_nav(siv, false);
        },
    }
}

fn get_entry_api_from_edit_view(siv: &mut Cursive) -> EntryAPI {
    let title = siv.find_name::<EditView>("TITLE")
        .expect("view: 'TITLE' not found")
        .get_content()
        .to_string();
    let description = siv.find_name::<TextArea>("DESCRIPTION")
        .expect("view: 'DESCRIPTION' not found")
        .get_content()
        .to_string();

    let hour_view = siv.find_name::<SelectView<i8>>("HOURS")
        .expect("view: 'HOURS' not found");

    let minute_view = siv.find_name::<SelectView<i8>>("MINUTES")
        .expect("view: 'MINUTES' not found");

    let hours_id = hour_view.selected_id().expect("no hour selected");
    let minutes_id = minute_view.selected_id().expect("no minute selected");

    let hours = hour_view.get_item(hours_id).expect("could not get item").1;
    let minutes = minute_view.get_item(minutes_id).expect("could not get item").1;

    let mut due_date = siv.find_name::<SelectView<Date<Local>>>("DATE_BUTTON")
        .expect("view: 'DATE_BUTTON' not found")
        .selection()
        .expect("nothing selected")
        .and_hms(*hours as u32, *minutes as u32, 1)
        .timestamp();

    if siv.find_name::<Checkbox>("DUE_DATE")
        .expect("view: 'DUE_DATE' not found")
        .is_checked()
        .not() {

        due_date = 0;
    }

    return EntryAPI {
        entryID: 0,
        creatorID: 0,
        createDate: 0,
        dueDate: due_date,
        title: title,
        description: description,
    };
}

fn create_entry_button_cb(siv: &mut Cursive, board_id: &i64) {
    let entry = get_entry_api_from_edit_view(siv);

    match create_entry(siv, entry, &board_id) {
        Ok(entry_id) => match get_entry_from_id(siv, entry_id) {
            Ok(entry) => load_to_entry_view(siv, entry),
            Err(error) => error_handler(siv, error),
        },
        Err(error) => error_handler(siv, error),
    }

    siv.pop_layer();
    set_callbacks(siv, true);
}

fn edit_entry_button_cb(siv: &mut Cursive, entry_id: &i64) {
    let mut entry = get_entry_api_from_edit_view(siv);
    entry.entryID = entry_id.clone();

    modify_entry(siv, entry);

    match get_entry_from_id(siv, entry_id.clone()) {
        Ok(entry) => match replace_in_entry_view(siv, entry) {
            Err(ErrorKind::NotFound) => notify_popup(siv, "not found", "entry not found"),
            _ => (),
        },
        Err(error) => error_handler(siv, error),
    }

    /*notify_popup(siv, "worked?", format!(
        "due date: {}, title: {}, description: {}",
        entry.dueDate, entry.title, entry.description
    ).as_str());*/

    set_callbacks(siv, true);
    siv.pop_layer();
}

fn modify_entry(siv: &mut Cursive, entry: EntryAPI) -> Result<(), BackendError> {
    let entry_id = entry.entryID;

    let token = &siv.user_data::<GlobalData>().expect("no token")
        .token
        .clone()
        .expect("clone failed");

    match siv.user_data::<GlobalData>()
        .expect("no user data set")
        .http_client
        .put(format!("{}/entry/{}", BASE_URL, entry_id))
        .header("token", token)
        .json::<EntryAPI>(&entry)
        .send() {

        Ok(response) =>
            if response.status().is_success() && response.status() != StatusCode::NO_CONTENT {
                return Ok(());
            } else {
                return Err(error_converter(response.status()))
            }
        Err(error) => panic!("{}", verbose_panic(error)),
    }
}

fn create_entry(siv: &mut Cursive, entry: EntryAPI, board_id: &i64) -> Result<i64, BackendError>{
    let token = &siv.user_data::<GlobalData>().expect("no token")
        .token
        .clone()
        .expect("clone failed");

    match siv.user_data::<GlobalData>()
        .expect("no user data set")
        .http_client
        .post(format!("{}/boards/{}/entry", BASE_URL, board_id))
        .header("token", token)
        .json::<EntryAPI>(&entry)
        .send() {

        Ok(response) =>
            if response.status().is_success() && response.status() != StatusCode::NO_CONTENT {
                return Ok(
                        response.json::<i64>()
                            .expect("didn't receive matching json object")
                );
            } else {
                return Err(error_converter(response.status()))
            }
        Err(error) => panic!("{}", verbose_panic(error)),
    }
}

fn on_submit_entry(siv: &mut Cursive, item: &EntryItem) {
    match item {
        EntryItem::Entry(entry) => {
            let entry_id = entry.entry_id.clone();
            edit_entry_popup(siv, "Edit entry", ("save", move |s|
                edit_entry_button_cb(s, &entry_id)
            ), Some(&entry.clone()))
        },
        EntryItem::Add(board_id) => {
            let board_id_c = board_id.clone();
            edit_entry_popup(siv, "Create entry", ("create", move |s|
                create_entry_button_cb(s, &board_id_c)
            ), None);
        },
    }
}

fn on_select_entry(siv: &mut Cursive, item: &EntryItem) {
    match item {
        EntryItem::Entry(entry) => {
            siv.find_name::<TextView>("ENTRY_DESCRIPTION")
                .expect("view: 'ENTRY_DESCRIPTION' not found")
                .set_content(entry.description.clone());
        },
        EntryItem::Add(_) => {
            siv.find_name::<TextView>("ENTRY_DESCRIPTION")
                .expect("view: 'ENTRY_DESCRIPTION' not found")
                .set_content("With this button you are able to create new entries.");
        },
    }
}

fn entry_api_to_entry(entry_api: EntryAPI) -> Entry {
    return Entry {
        entry_id: entry_api.entryID,
        creator_id: entry_api.creatorID,
        create_date: chrono::DateTime::from(
            chrono::DateTime::<chrono::Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp(
                    entry_api.createDate, 0
                ),
                chrono::Utc
            )
        ),
        due_date: chrono::DateTime::from(
            chrono::DateTime::<chrono::Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp(
                    entry_api.dueDate, 0
                ),
                chrono::Utc
            )
        ),
        title: entry_api.title,
        description: entry_api.description,
    };
}

fn board_api_to_board(board_api: BoardAPI) -> Board {
    return Board {
        board_id: board_api.boardID,
        name: board_api.name,
        create_date: chrono::DateTime::from(
            chrono::DateTime::<chrono::Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp(
                    board_api.createDate, 0
                ),
                chrono::Utc
            )
        ),
        creator_id: board_api.creatorID,
    };
}

fn clear_entry_view(siv: &mut Cursive) {
    siv.find_name::<SelectView<EntryItem>>("ENTRY_SELECTION")
        .expect("view: 'ENTRY_SELECTION' not found")
        .clear();
}

fn load_to_entry_view(siv: &mut Cursive, entry: Entry) {
    siv.find_name::<SelectView<EntryItem>>("ENTRY_SELECTION")
        .expect("view: 'ENTRY_SELECTION' not found")
        .insert_item(0, entry.title.clone(), EntryItem::Entry(entry));
}

fn replace_in_entry_view(siv: &mut Cursive, entry: Entry) -> Result<(), ErrorKind> {
    let mut entry_view = siv.find_name::<SelectView<EntryItem>>("ENTRY_SELECTION")
        .expect("view: 'ENTRY_SELECTION' not found");

    let entry_id = entry.entry_id;

    let entry_index = entry_view.iter().position(|item| {
        match item.1 {
            EntryItem::Entry(entry) => entry.entry_id == entry_id,
            _ => false,
        }
    });

    if let Some(index) = entry_index {
        entry_view.remove_item(index);
        entry_view.insert_item(
            index,
            entry.title.clone(),
            EntryItem::Entry(entry)
        );
        return Ok(());
    } else {
        return Err(ErrorKind::NotFound);
    }
}

fn switch_stack(siv: &mut Cursive, stack_name: &str, layer_name: &str) {
    let mut stack = siv.find_name::<StackView>(stack_name) // = "BOARD_STACK"
        .expect("view not found");

    let stack_position = stack.find_layer_from_name(layer_name)
        .expect("layer not found"); // = "ENTRY_LAYER"

    stack.move_to_front(stack_position);
}

fn load_to_board_view(siv: &mut Cursive, board: Board) {
    siv.find_name::<SelectView<BoardItem>>("BOARD_SELECTION")
        .expect("view: 'BOARD_SELECTION' not found")
        .insert_item(0, board.name.clone(), BoardItem::Board(board));
}

fn error_handler(siv: &mut Cursive, error: BackendError) {
    match error {
        BackendError::Incomplete => notify_popup(
            siv,
            "BAD_REQUEST",
            "The request body format probably changed for an endpoint, so if you see this, open an issue"
        ),
        BackendError::TokenInvalid => {
            notify_popup(
                siv,
                "Session Expired",
                "Your session expired, you need to re-login"
            );
            //logout function should be here
        },
        BackendError::Deleted => {
            notify_popup(
                siv,
                "Entry doesn't exist",
                "The entry was probably delete right when you opened it, you should try reloading"
            )
            //reload function should be here
        },
        BackendError::NoAccess => {
                notify_popup(
                    siv,
                    "FORBIDDEN",
                    "A request returned a 403: FORBIDDEN, this shouldn't ever happen, so if you see this, open an issue"
                );
        }
    }
}

fn error_converter(status_code: reqwest::StatusCode) -> BackendError {
    if status_code == StatusCode::UNAUTHORIZED {
        return BackendError::TokenInvalid;
    } else if status_code == StatusCode::NO_CONTENT {
        return BackendError::Deleted;
    } else if status_code == StatusCode::FORBIDDEN {
        return BackendError::NoAccess;
    } else if status_code == StatusCode::INTERNAL_SERVER_ERROR {
        panic!("haha jakobs fehler lol")
    } else if status_code == StatusCode::BAD_REQUEST {
        panic!("made a bad request")
    } else {
        panic!("server returned an unexpected status");
    }
}

fn get_board_from_id(siv: &mut Cursive, board_id: i64) -> Result<Board, BackendError>{
    let token = &siv.user_data::<GlobalData>().expect("no token")
        .token
        .clone()
        .expect("clone failed");

    match siv.user_data::<GlobalData>()
        .expect("no user data set")
        .http_client
        .get(format!("{}/boards/{}", BASE_URL, board_id))
        .header("token", token)
        .send() {

        Ok(response) =>
            if response.status().is_success() && response.status() != StatusCode::NO_CONTENT {
                return Ok(
                    board_api_to_board(
                        response.json::<BoardAPI>()
                            .expect("didn't receive matching json object")
                    )
                );
            } else {
                return Err(error_converter(response.status()))
            }
        Err(error) => panic!("{}", verbose_panic(error)),
    }
}

fn verbose_panic(error: reqwest::Error) -> String {
    return format!(
        "request errored, the error is: {} and the url: {}",
        error.to_string(),
        error.url().unwrap()
    );
}

/*fn ask_date(siv: &mut Cursive, select_name: &str) {
    siv.add_layer(
        Dialog::new()
            .content(
                CalendarView::<Local, EnglishLocale>::new(
                    chrono::Date::from(
                        chrono::offset::Local::today()
                    )
                ).view_mode(ViewMode::Month)
                    .on_submit(|s, v| )
            )
    )
}*/

fn open_calendar(siv: &mut Cursive, item: &Date<Local>, date_button: String) {
    siv.add_layer(
        Dialog::new()
            .content(
                CalendarView::<Local, EnglishLocale>::new(item.clone()).view_mode(ViewMode::Month)
                    .on_submit(move |s, v| {
                        let mut button = s.find_name::<SelectView<Date<Local>>>(date_button.as_str())
                            .expect(format!("view: '{}' not found", date_button).as_str());

                        button.clear();

                        button.add_item(v.format(" %d.%m.%Y ").to_string(), v.clone());

                        s.pop_layer();
                    })
            )
    )
}

fn edit_entry_popup<F>(siv: &mut Cursive, title: &str, button: (&str, F), entry: Option<&Entry>)
where
    F: 'static + Fn(&mut Cursive),
{

    fn change_due_date_state(s: &mut Cursive, state: bool) {
        let mut date =
            s.find_name::<SelectView<Date<Local>>>("DATE_BUTTON")
                .expect("view: 'DATE_BUTTON' not found");

        let mut hours =
            s.find_name::<SelectView<i8>>("HOURS")
                .expect("view: 'HOURS' not found");

        let mut minutes =
            s.find_name::<SelectView<i8>>("MINUTES")
                .expect("view: 'MINUTES' not found");

        if state {
            date.enable();
            hours.enable();
            minutes.enable();
        } else {
            date.disable();
            hours.disable();
            minutes.disable();
        }
    }

    set_callbacks(siv, false);

    let mut time = chrono::DateTime::from(chrono::offset::Local::now());
    let mut title_entry = "";
    let mut description = "";

    if let Some(entry_obj) = entry {
        time = entry_obj.due_date;
        title_entry = &entry_obj.title;
        description = &entry_obj.description;
    }

    let hours_view: SelectView<i8> = SelectView::new()
        .autojump()
        .popup();

    let mut c: i8 = -1;
    let hour_items = vec![(0, 0); 24].into_iter().map(|_| {
        c += 1;
        return (format!("{:02}", c as i8), c);
    });

    let minutes_view: SelectView<i8> = SelectView::new()
        .autojump()
        .popup();

    let mut c = -5;
    let minute_items = vec![(0, 0); 12].into_iter().map(|_| {
        c += 5;
        return (format!("{:02}", c as i8), c);
    });

    siv.add_layer(
        Dialog::new()
        .content(
            LinearLayout::vertical()
                .child(
                    TextView::new("\nTitle")
                        .fixed_height(2)
                )
                .child(
                    EditView::new()
                        .content(title_entry)
                        .with_name("TITLE")
                )
                .child(
                    TextView::new("\nDescription")
                        .fixed_height(2)
                )
                .child(
                    TextArea::new()
                        .content(description)
                        .with_name("DESCRIPTION")
                        .fixed_height(5)
                )
                .child(
                    LinearLayout::horizontal()
                        .child(
                            PaddedView::lrtb(0, 2, 1, 1,
                                 LinearLayout::vertical()
                                     .child(
                                         TextView::new("Date")
                                     )
                                     .child(
                                         SelectView::new()
                                             .item(
                                                 time.date().format(" %d.%m.%Y ")
                                                     .to_string(),
                                                 time.date()
                                             )
                                             .on_submit(
                                                 |s, i|
                                                     open_calendar(
                                                         s,
                                                         i,
                                                         String::from("DATE_BUTTON")
                                                     )
                                             )
                                             .disabled()
                                             .with_name("DATE_BUTTON")
                                     )
                            )
                        )
                        .child(
                            PaddedView::lrtb(2, 0, 1, 1,
                                LinearLayout::vertical()
                                    .child(
                                        TextView::new("Time")
                                    )
                                    .child(
                                        LinearLayout::horizontal()
                                            .child(
                                                hours_view.with_all(
                                                    hour_items.into_iter()
                                                ).disabled()
                                                    .with_name("HOURS")
                                            )
                                            .child(
                                                TextView::new( ":")
                                            )
                                            .child(
                                                minutes_view.with_all(
                                                    minute_items.into_iter()
                                                ).disabled()
                                                    .with_name("MINUTES")
                                            )
                                    )
                            )
                        )
                )
                .child(
                    LinearLayout::horizontal()
                        .child(
                            Checkbox::new()
                                .on_change(change_due_date_state)
                                .with_name("DUE_DATE")
                        )
                        .child(
                            TextView::new(" with due date")
                        )
                )
        )
        .title(title)
        .button(
            "cancel",
            |s| {
                set_callbacks(s, true);
                s.pop_layer();
            }
        )
        .button(button.0,  button.1)
    );

    if let Some(entry) = entry {
        if (entry.due_date.timestamp() == 0).not() {
            siv.find_name::<Checkbox>("DUE_DATE")
                .expect("view: 'DUE_DATE' not found")
                .set_checked(true);

            change_due_date_state(siv, true);
        }
    }

    let mut hour_view = siv.find_name::<SelectView<i8>>("HOURS")
        .expect("view: 'HOURS' not found");

    let mut minute_view = siv.find_name::<SelectView<i8>>("MINUTES")
        .expect("view: 'MINUTES' not found");

    if let Some(_) = entry {
        //update time view
        let hour = time.hour() as i8;
        let minute = (((time.minute() + 5) / 5 - 1) as i8) * 5;

        let hour_position = hour_view.iter()
            .position(|item| item.1 == &hour)
            .expect("iteration find failed");

        let minute_position = minute_view.iter()
            .position(|item| item.1 == &minute)
            .expect("iteration find failed");

        hour_view.set_selection(hour_position);
        minute_view.set_selection(minute_position);
    }

    //move to the button closure
    /*{
        //update time
        let hours_id = hour_view.selected_id().expect("no hour selected");
        let minutes_id = minute_view.selected_id().expect("no minute selected");

        let hours = hour_view.get_item(hours_id).expect("could not get item").1;
        let minutes = minute_view.get_item(minutes_id).expect("could not get item").1;

        time = time.with_hour(hours as u32)
            .expect("out of range")
            .with_minute(minutes as u32)
            .expect("out of range");
    }*/
}

fn get_board_entry_ids(siv: &mut Cursive, board_id: i64) -> Result<vec::Vec<i64>, BackendError> { //board 8
    let token = &siv.user_data::<GlobalData>().expect("no token")
        .token
        .clone()
        .expect("clone failed");

    match siv.user_data::<GlobalData>()
        .expect("no user data set")
        .http_client
        .get(format!("{}/boards/{}/entries", BASE_URL, board_id))
        .header("token", token).send() {
        Ok(response) => if response.status().is_success() {
            return Ok(response.json::<vec::Vec<i64>>().expect("didn't receive json array of i64's"));
        } else {
            return Err(error_converter(response.status()))
        },
        Err(error) => panic!("{}", verbose_panic(error)),
    }
}

fn get_entry_from_id(siv: &mut Cursive, entry_id: i64) -> Result<Entry, BackendError> {
    let token = &siv.user_data::<GlobalData>().expect("no token")
        .token
        .clone()
        .expect("clone failed");

    match siv.user_data::<GlobalData>()
        .expect("no user data set")
        .http_client
        .get(format!("{}/entry/{}", BASE_URL, entry_id))
        .header("token", token)
        .send() {

        Ok(response) => if response.status().is_success() && response.status() != StatusCode::NO_CONTENT {
            return Ok(
                entry_api_to_entry(
                    response.json::<EntryAPI>()
                        .expect("didn't receive matching json object")
                )
            );
        } else {
            return Err(error_converter(response.status()))
        }
        Err(error) => panic!("{}", verbose_panic(error)),
    }
}

fn get_board_ids(siv: &mut Cursive) -> Result<vec::Vec<i64>, BackendError> {
    let token = &siv.user_data::<GlobalData>().expect("no token")
        .token
        .clone()
        .expect("clone failed");

    match siv.user_data::<GlobalData>()
        .expect("no user data set")
        .http_client
        .get(format!("{}/boards/user", BASE_URL))
        .header("token", token).send() {
        Ok(response) => if response.status().is_success() {
            return Ok(response.json::<vec::Vec<i64>>().expect("didn't receive json array of i64's"));
        } else {
            return Err(error_converter(response.status()))
        },
        Err(error) => panic!("{}", verbose_panic(error)),
    }
}

fn exit(siv: &mut Cursive) {
    siv.quit();
}

fn login_page(siv: &mut Cursive) {
    siv.pop_layer();
    siv.add_layer(Dialog::new()
        .title("Login - YAP")
        .content(
        LinearLayout::vertical()
            .child(
                TextView::new("\nemail:")
                    .fixed_height(2)
            )
            .child(
                EditView::new()
                    .with_name("EMAIL_LOGIN")
                    .fixed_width(34)
            )
            .child(
                TextView::new("\npassword:")
                    .fixed_height(2)
            )
            .child(
                EditView::new()
                    .secret()
                    .with_name("PASSWORD_LOGIN")
                    .fixed_width(34)
            )
            .child(
                TextView::new("\n")
            )
            .child(
            LinearLayout::horizontal()
                .child(
                    Checkbox::new()
                        .on_change(|siv, state|
                            if let Some(mut view) =
                            siv.find_name::<EditView>("PASSWORD_LOGIN") {
                                view.set_secret(state.not());
                            }
                        )
                )
                .child(
                    TextView::new(" Show password")
                        .fixed_width(16)
                )
                .child(
                    Checkbox::new()
                        .with_name("REMEMBER_ME_LOGIN")
                )
                .child(
                    TextView::new(" Remember Me")
                )
            )
        )
        .button("Back", |siv| welcome_page(siv))
        .button("Login", login)
    );
}

fn login(siv: &mut Cursive) {

    let email = siv.find_name::<EditView>("EMAIL_LOGIN")
        .unwrap_or_else(
            || siv.find_name::<EditView>("EMAIL_REGISTER")
                .expect("couldn't find view by name"))
        .get_content();

    let password = siv.find_name::<EditView>("PASSWORD_LOGIN")
        .unwrap_or_else(
            || siv.find_name::<EditView>("PASSWORD_REGISTER")
                .expect("couldn't find view by name"))
        .get_content();

    //Send request to backend to obtain a token
    match siv.user_data::<GlobalData>()
        .expect("no user data set")
        .http_client.post(format!("{}/security/token", BASE_URL))
        .header("content-type", "application/json")
        .body(format!(
            "{{\"emailAddress\":\"{}\",\"password\":\"{}\"}}",
            email,
            password
        ))
        .send() {

        Ok(request) => { // <- If the status code is an Error it will still return an Ok()
            if request.status().is_success() {
                remove_file(siv, TOKEN_FILE);

                siv.with_user_data(|data: &mut GlobalData | {
                    data.token = Some(request.text().unwrap());
                });

                //Write the token to a file if REMEMBER_ME is checked
                if let Some(state) = siv.find_name::<Checkbox>(
                    "REMEMBER_ME_LOGIN"
                ) {
                    if state.is_checked().eq(&true) {

                        create_file(siv, TOKEN_FILE);

                        if let Ok(mut file) = get_file(siv, TOKEN_FILE) {
                            file.write_all(
                                serde_json::to_string_pretty(
                                    &TokenFile {
                                        user_mail: email.to_string(),
                                        token: siv.user_data::<GlobalData>()
                                            .expect("no user data set")
                                            .token
                                            .as_ref()
                                            .unwrap()
                                            .to_string(),
                                    })
                                    .unwrap()
                                    .as_bytes()
                            ).expect("couldn't write to token file");
                        } else {
                            notify_popup(siv, "Remember Me", "Remember Me didn't work :(")
                        }
                    }
                } else {
                    notify_popup(siv, "No success!", "file not created");
                }
                main_screen(siv);
            } else {
                if request.status() == StatusCode::FORBIDDEN {
                    notify_popup(
                        siv,
                        "Wrong credentials!",
                        "Either your mail or password is wrong."
                    );
                } else {
                    notify_popup(
                        siv,
                        "Request failed.",
                        &*format!(
                            "Request failed with code: {}",
                            request.status().as_str()
                        )
                    )
                }
            }
        },
        Err(error) => {
            if let Some(status) = error.status() {
                notify_popup(siv, "Request failed.",
                             &*format!("Request failed with code: {}", status))
            } else {
                notify_popup(siv, "Request failed.", "Reason: Unknown");
            }
        },
    }
}


fn main_screen(siv: &mut Cursive) {
    siv.pop_layer();

    siv.add_fullscreen_layer(
        LinearLayout::vertical()
            .child(
                ResizedView::new(
                    SizeConstraint::Full,
                    SizeConstraint::Fixed(1),
                    LinearLayout::horizontal()
                        .child(
                            ResizedView::with_full_screen(
                                Layer::with_color(
                                    TextView::new("Boards")
                                        .h_align(HAlign::Center)
                                        .style(ColorStyle::highlight()),
                                    ColorStyle::highlight()
                                ).with_name(TABS[0].indicator)
                            )
                        )
                        .child(
                            ResizedView::with_full_screen(
                                Layer::new(
                                    TextView::new("Account")
                                        .h_align(HAlign::Center)
                                ).with_name(TABS[1].indicator)
                            )
                        )
                )
            )
            .child(
                LinearLayout::vertical()
                    .child(
                        StackView::new()
                            .fullscreen_layer(
                                ResizedView::with_full_screen(
                                    LinearLayout::vertical()
                                        .child(
                                            TextView::new("Password")
                                        )
                                        .child(
                                            EditView::new()
                                        )
                                ).with_name(TABS[1].layer)
                            )
                            .fullscreen_layer(
                                StackView::new()
                                    .fullscreen_layer(
                                        LinearLayout::horizontal()
                                            .child(
                                                ResizedView::with_full_screen(
                                                    ScrollView::new(
                                                        ResizedView::with_full_screen(
                                                            SelectView::new()
                                                                .autojump()
                                                                .on_submit(on_submit_entry)
                                                                .on_select(on_select_entry)
                                                                .with_name("ENTRY_SELECTION")
                                                        )
                                                    )
                                                )
                                            )
                                            .child(
                                                ResizedView::with_full_screen(
                                                    Panel::new(
                                                        ScrollView::new(
                                                            TextView::new("TextText on new line")
                                                                .with_name("ENTRY_DESCRIPTION")
                                                        )
                                                    ).title("Description")
                                                        .title_position(HAlign::Center)
                                                )
                                            ).with_name("ENTRY_LAYER")
                                    )
                                    .fullscreen_layer(
                                        ResizedView::with_full_screen(
                                            ScrollView::new(
                                                ResizedView::with_full_screen(
                                                    SelectView::new()
                                                        .autojump()
                                                        .item("<add new board>", BoardItem::Add)
                                                        .on_submit(on_submit_board)
                                                        .with_name("BOARD_SELECTION")
                                                )
                                            ).with_name(TABS[0].layer)
                                        ).with_name("BOARD_LAYER")
                                    ).with_name("BOARD_STACK")
                            ).with_name("TAB_LAYERS")

                    )
                    /*.child(
                        ResizedView::new(
                            SizeConstraint::Full,
                            SizeConstraint::Fixed(1),
                            Button::new("reload", reload_all)
                        )
                    )*/
            )
    );

    set_callbacks(siv, true);

    load_boards_to_view(siv);

    //edit_entry_popup(siv, "Create new entry");
}

fn register_page(siv: &mut Cursive) {

    siv.pop_layer();
    siv.add_layer(Dialog::new()
        .title("Register - YAP")
        .content(
        LinearLayout::vertical()
            .child(
                TextView::new("\nusername:")
                    .fixed_height(2)
            )
            .child(
                EditView::new()
                    .with_name("USERNAME_REGISTER")
                    .fixed_width(32)
            )
            .child(
                TextView::new("\nemail:")
                    .fixed_height(2)
            )
            .child(
                EditView::new()
                    .with_name("EMAIL_REGISTER")
                    .fixed_width(32)
            )
            .child(
                TextView::new("\npassword:")
                    .fixed_height(2)
            )
            .child(
                EditView::new()
                    .secret()
                    .with_name("PASSWORD_REGISTER")
                    .fixed_width(32)
            )
            .child(
                TextView::new("\nretype password:")
                    .fixed_height(2)
            )
            .child(
                EditView::new()
                    .secret()
                    .with_name("PASSWORD_CHECK_REGISTER")
                    .fixed_width(32)
            )
            .child(
                TextView::new("\n")
            )
            .child(
            LinearLayout::horizontal()
                .child(
                Checkbox::new()
                    .on_change(|siv, state|
                        if let Some(mut view) =
                        siv.find_name::<EditView>("PASSWORD_REGISTER") {
                            if let Some(mut check_view) =
                            siv.find_name::<EditView>("PASSWORD_CHECK_REGISTER") {

                                view.set_secret(state.not());
                                check_view.set_secret(state.not());
                            }
                        }
                    )
                )
                .child(
                    TextView::new(" Show password")
                )
            )
        )
        .button("Back", |siv| welcome_page(siv))
        .button("Register and login", |siv| {
                match check_register(siv) {
                    Ok(_) => register(siv),
                    Err(RegisterInvalid::InvalidUsername) =>
                        notify_popup(siv, "credentials not valid",
                                     "error: username not valid"),
                    Err(RegisterInvalid::InvalidEmail) =>
                        notify_popup(siv, "credentials not valid",
                                     "error: email not valid"),
                    Err(RegisterInvalid::InvalidPassword) =>
                        notify_popup(siv, "credentials not valid",
                                     "error: password not valid"),
            }
        })
    );
}

fn notify_popup(siv: &mut Cursive, title: &str, message: &str) {
    siv.add_layer(
        Dialog::text(message)
            .title(title)
            .dismiss_button("Ok")
    );
}

fn check_register(siv: &mut Cursive) -> Result<(), RegisterInvalid> {
    let username = siv.find_name::<EditView>("USERNAME_REGISTER")
        .expect("couldn't find view by name");

    let email = siv.find_name::<EditView>("EMAIL_REGISTER")
        .expect("couldn't find view by name");

    let password = siv.find_name::<EditView>("PASSWORD_REGISTER")
        .expect("couldn't find view by name");

    let password_check = siv.find_name::<EditView>("PASSWORD_CHECK_REGISTER")
        .expect("couldn't find view by name");


    if username.get_content().as_str().eq("") ||
        username.get_content().len() > 32 {

        return Err(RegisterInvalid::InvalidUsername);
    }

    if email.get_content().as_str().eq("") ||
        Regex::new(/*<editor-fold desc="email regEx">*/"(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|\"(?:[\\x01-\\x08\\x0b\\x0c\\x0e-\\x1f\\x21\\x23-\\x5b\\x5d-\\x7f]|\\\\[\\x01-\\x09\\x0b\\x0c\\x0e-\\x7f])*\")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\\[(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?|[a-z0-9-]*[a-z0-9]:(?:[\\x01-\\x08\\x0b\\x0c\\x0e-\\x1f\\x21-\\x5a\\x53-\\x7f]|\\\\[\\x01-\\x09\\x0b\\x0c\\x0e-\\x7f])+)\\])"
                   /*</editor-fold>*/).unwrap().is_match(
            email.get_content().as_str()
        ).not() {

        return Err(RegisterInvalid::InvalidEmail);
    }

    if password.get_content().len() < 10 ||
        password.get_content().eq(&password_check.get_content()).not() ||
        password.get_content().len() > 1024 {

        return Err(RegisterInvalid::InvalidPassword);
    }
    return Ok(());
}

fn register(siv: &mut Cursive) {
    match siv.user_data::<GlobalData>()
        .expect("no user data set")
        .http_client
        .post(format!("{}/user", BASE_URL))
        .header("content-type", "application/json")
        .body(format!(
            "{{\"username\":\"{}\",\"emailAddress\":\"{}\",\"password\":\"{}\"}}",
            siv.find_name::<EditView>("USERNAME_REGISTER")
                .expect("couldn't find view by name").
                get_content(),
            siv.find_name::<EditView>("EMAIL_REGISTER")
                .expect("couldn't find view by name")
                .get_content(),
            siv.find_name::<EditView>("PASSWORD_REGISTER")
                .expect("couldn't find view by name")
                .get_content()
        ))
        .send() {
        Ok(_) => {
            notify_popup(siv, "Success!", "Successfully created user");
        },
        Err(error) => {
            if let Some(status) = error.status() {
                notify_popup(siv, "Request failed.",
                             &*format!("Request failed with code: {}", status))
            } else {
                notify_popup(siv, "Request failed.", "Reason: Unknown");
            }
        },
    }
    siv.pop_layer();
    login(siv);
}

fn get_path(siv: &mut Cursive, file: &str) -> Result<PathBuf, std::io::ErrorKind> {
    if let Some(path) = &siv.user_data::<GlobalData>()
        .expect("no user data set")
        .config_home
        .find_config_file(file) {
        return Ok(path.clone());
    } else {
        return Err(std::io::ErrorKind::NotFound);
    }
}

fn get_file(siv: &mut Cursive, file: &str) -> Result<File, std::io::ErrorKind> {
    if let Ok(path) = get_path(siv, file) {
        return Ok(OpenOptions::new()
            .write(true)
            .open(path)
            .expect("file couldn't be opened"));
    } else {
        return Err(std::io::ErrorKind::NotFound);
    }
}

//TODO add Result<(), Error>
fn remove_file(siv: &mut Cursive, file: &str) {
    if let Some(file_path) = &siv.user_data::<GlobalData>()
        .expect("no user data set")
        .config_home
        .find_config_file(file) {

        fs::remove_file(file_path)
            .expect(
                &*format!(
                    "couldn't remove {} file",
                    file_path.to_str().unwrap()
                )
            );
    }
}

//TODO add Result<(), Error>
fn create_file(siv: &mut Cursive, file: &str) {
    match siv.user_data::<GlobalData>()
        .expect("no user data set")
        .config_home
        .place_config_file(file) {
        Ok(file_path) => if Path::exists(file_path.as_ref()).not() {
            File::create(file_path).expect("couldn't create file");
        }
        Err(error) => panic!("{}", error.to_string()),
    }
}

fn welcome_page(siv: &mut Cursive) {
    siv.pop_layer();
    siv.add_layer(Dialog::text(
        "Welcome to YAP!\nPress <Login> if you already have an account, \
         else consider creating one by pressing <Register>"
    )
        .title("Login - YAP")
        .button("Quit", |siv| siv.quit())
        .button("Login", login_page)
        .button("Register", register_page));
}

fn check_token(siv: &mut Cursive, token: &str) -> bool {
    if let Ok(response) = siv.user_data::<GlobalData>()
        .expect("no user data set")
        .http_client
        .get(format!("{}/security/token/checkValid", BASE_URL))
        .header("token", token)
        .send() {
        if let Ok(status) = response.text().unwrap().parse::<bool>() {
            return status;
        } else {
            return false;
        }
    } else {
        return false;
    }
}

//TODO change error type to a generic one?
fn load_token(siv: &mut Cursive) -> Result<TokenFile, TokenLoadError> {
    if let Ok(path) = get_path(siv, TOKEN_FILE) {
        if let Ok(token_content) = fs::read_to_string(path) {
            if let Ok(token_struct) = serde_json::from_str::<TokenFile>(&*token_content) {
                if check_token(siv, &token_struct.token) {
                    return Ok(token_struct);
                } else {
                    return Err(TokenLoadError::TokenExpired);
                }
            } else {
                return Err(TokenLoadError::FileNotReadable);
            }
        } else {
            return Err(TokenLoadError::FileNotReadable);
        }
    } else {
        return Err(TokenLoadError::FileNotFound);
    }
}


fn main() {
    //initialize objects
    let mut siv = cursive::default();

    //bind exit to 'q' to be able to exit at any time
    siv.add_global_callback('q', exit);

    siv.add_global_callback('\\', Cursive::toggle_debug_console);

    siv.set_user_data(GlobalData {
        token: None,
        http_client: blocking::Client::new(),
        config_home: xdg::BaseDirectories::with_prefix("yap").unwrap(),
    });

    //load theme file if present
    if let Ok(file) = get_path(&mut siv, "theme.toml") {
        siv.load_theme_file(file).unwrap();
    }

    //siv.user_data::<GlobalData>().unwrap().config_home.place_data_file(TOKEN_FILE).expect("token file not placed");
    //siv.with_user_data(|data: &mut GlobalData| data.config_home.place_data_file(TOKEN_FILE).expect("couldn't place token file"));

    //display the welcome page
    //TODO integrate this mess into the login function, which should by then use the credentials as parameters
    if let Ok(token_comb) = load_token(&mut siv) {
        siv.add_layer(Dialog::text(
            format!("Is {} you?", token_comb.user_mail))
            .button("yes", move |mut siv| {

                //let http_client = blocking::Client::new();

                /*siv.set_user_data(
                    GlobalData {
                        token: Some(token_comb.token.clone()),
                        http_client,

                    }
                );*/

                siv.with_user_data(|data: &mut GlobalData| {
                    data.token = Some(token_comb.token.clone());
                });

                main_screen(&mut siv);
            })
            .button("no", |siv| {
                remove_file(siv, TOKEN_FILE);
                login_page(siv);
            })
        );
    } else {
        welcome_page(&mut siv);
    }

    //start the event loop
    siv.run();
}
