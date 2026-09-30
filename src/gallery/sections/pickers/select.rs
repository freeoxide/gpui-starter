//! Select section, ported from the upstream `SelectStory`.

use gpui_kit::component::{
    ActiveTheme as _, IconName, IndexPath, Sizable as _, Size,
    button::{Button, ButtonVariants as _, DropdownButton},
    h_flex,
    input::{Input, InputState},
    select::*,
    separator::Separator,
    v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

// Ported from the story crate's countries.json fixture; the gallery module
// must stay self-contained, so the data lives here instead of include_str!.
const COUNTRIES: &[(&str, &str)] = &[
    ("Afghanistan", "AF"),
    ("Åland Islands", "AX"),
    ("Albania", "AL"),
    ("Algeria", "DZ"),
    ("American Samoa", "AS"),
    ("AndorrA", "AD"),
    ("Angola", "AO"),
    ("Anguilla", "AI"),
    ("Antarctica", "AQ"),
    ("Antigua and Barbuda", "AG"),
    ("Argentina", "AR"),
    ("Armenia", "AM"),
    ("Aruba", "AW"),
    ("Australia", "AU"),
    ("Austria", "AT"),
    ("Azerbaijan", "AZ"),
    ("Bahamas", "BS"),
    ("Bahrain", "BH"),
    ("Bangladesh", "BD"),
    ("Barbados", "BB"),
    ("Belarus", "BY"),
    ("Belgium", "BE"),
    ("Belize", "BZ"),
    ("Benin", "BJ"),
    ("Bermuda", "BM"),
    ("Bhutan", "BT"),
    ("Bolivia", "BO"),
    ("Bosnia and Herzegovina", "BA"),
    ("Botswana", "BW"),
    ("Bouvet Island", "BV"),
    ("Brazil", "BR"),
    ("British Indian Ocean Territory", "IO"),
    ("Brunei Darussalam", "BN"),
    ("Bulgaria", "BG"),
    ("Burkina Faso", "BF"),
    ("Burundi", "BI"),
    ("Cambodia", "KH"),
    ("Cameroon", "CM"),
    ("Canada", "CA"),
    ("Cape Verde", "CV"),
    ("Cayman Islands", "KY"),
    ("Central African Republic", "CF"),
    ("Chad", "TD"),
    ("Chile", "CL"),
    ("China", "CN"),
    ("Christmas Island", "CX"),
    ("Cocos (Keeling) Islands", "CC"),
    ("Colombia", "CO"),
    ("Comoros", "KM"),
    ("Congo", "CG"),
    ("Congo, The Democratic Republic of the", "CD"),
    ("Cook Islands", "CK"),
    ("Costa Rica", "CR"),
    ("Cote D'Ivoire", "CI"),
    ("Croatia", "HR"),
    ("Cuba", "CU"),
    ("Cyprus", "CY"),
    ("Czech Republic", "CZ"),
    ("Denmark", "DK"),
    ("Djibouti", "DJ"),
    ("Dominica", "DM"),
    ("Dominican Republic", "DO"),
    ("Ecuador", "EC"),
    ("Egypt", "EG"),
    ("El Salvador", "SV"),
    ("Equatorial Guinea", "GQ"),
    ("Eritrea", "ER"),
    ("Estonia", "EE"),
    ("Ethiopia", "ET"),
    ("Falkland Islands (Malvinas)", "FK"),
    ("Faroe Islands", "FO"),
    ("Fiji", "FJ"),
    ("Finland", "FI"),
    ("France", "FR"),
    ("French Guiana", "GF"),
    ("French Polynesia", "PF"),
    ("French Southern Territories", "TF"),
    ("Gabon", "GA"),
    ("Gambia", "GM"),
    ("Georgia", "GE"),
    ("Germany", "DE"),
    ("Ghana", "GH"),
    ("Gibraltar", "GI"),
    ("Greece", "GR"),
    ("Greenland", "GL"),
    ("Grenada", "GD"),
    ("Guadeloupe", "GP"),
    ("Guam", "GU"),
    ("Guatemala", "GT"),
    ("Guernsey", "GG"),
    ("Guinea", "GN"),
    ("Guinea-Bissau", "GW"),
    ("Guyana", "GY"),
    ("Haiti", "HT"),
    ("Heard Island and Mcdonald Islands", "HM"),
    ("Holy See (Vatican City State)", "VA"),
    ("Honduras", "HN"),
    ("Hong Kong", "HK"),
    ("Hungary", "HU"),
    ("Iceland", "IS"),
    ("India", "IN"),
    ("Indonesia", "ID"),
    ("Iran, Islamic Republic Of", "IR"),
    ("Iraq", "IQ"),
    ("Ireland", "IE"),
    ("Isle of Man", "IM"),
    ("Israel", "IL"),
    ("Italy", "IT"),
    ("Jamaica", "JM"),
    ("Japan", "JP"),
    ("Jersey", "JE"),
    ("Jordan", "JO"),
    ("Kazakhstan", "KZ"),
    ("Kenya", "KE"),
    ("Kiribati", "KI"),
    ("Korea, Democratic People'S Republic of", "KP"),
    ("Korea, Republic of", "KR"),
    ("Kuwait", "KW"),
    ("Kyrgyzstan", "KG"),
    ("Lao People'S Democratic Republic", "LA"),
    ("Latvia", "LV"),
    ("Lebanon", "LB"),
    ("Lesotho", "LS"),
    ("Liberia", "LR"),
    ("Libyan Arab Jamahiriya", "LY"),
    ("Liechtenstein", "LI"),
    ("Lithuania", "LT"),
    ("Luxembourg", "LU"),
    ("Macao", "MO"),
    ("Macedonia, The Former Yugoslav Republic of", "MK"),
    ("Madagascar", "MG"),
    ("Malawi", "MW"),
    ("Malaysia", "MY"),
    ("Maldives", "MV"),
    ("Mali", "ML"),
    ("Malta", "MT"),
    ("Marshall Islands", "MH"),
    ("Martinique", "MQ"),
    ("Mauritania", "MR"),
    ("Mauritius", "MU"),
    ("Mayotte", "YT"),
    ("Mexico", "MX"),
    ("Micronesia, Federated States of", "FM"),
    ("Moldova, Republic of", "MD"),
    ("Monaco", "MC"),
    ("Mongolia", "MN"),
    ("Montserrat", "MS"),
    ("Morocco", "MA"),
    ("Mozambique", "MZ"),
    ("Myanmar", "MM"),
    ("Namibia", "NA"),
    ("Nauru", "NR"),
    ("Nepal", "NP"),
    ("Netherlands", "NL"),
    ("Netherlands Antilles", "AN"),
    ("New Caledonia", "NC"),
    ("New Zealand", "NZ"),
    ("Nicaragua", "NI"),
    ("Niger", "NE"),
    ("Nigeria", "NG"),
    ("Niue", "NU"),
    ("Norfolk Island", "NF"),
    ("Northern Mariana Islands", "MP"),
    ("Norway", "NO"),
    ("Oman", "OM"),
    ("Pakistan", "PK"),
    ("Palau", "PW"),
    ("Palestinian Territory, Occupied", "PS"),
    ("Panama", "PA"),
    ("Papua New Guinea", "PG"),
    ("Paraguay", "PY"),
    ("Peru", "PE"),
    ("Philippines", "PH"),
    ("Pitcairn", "PN"),
    ("Poland", "PL"),
    ("Portugal", "PT"),
    ("Puerto Rico", "PR"),
    ("Qatar", "QA"),
    ("Reunion", "RE"),
    ("Romania", "RO"),
    ("Russian Federation", "RU"),
    ("RWANDA", "RW"),
    ("Saint Helena", "SH"),
    ("Saint Kitts and Nevis", "KN"),
    ("Saint Lucia", "LC"),
    ("Saint Pierre and Miquelon", "PM"),
    ("Saint Vincent and the Grenadines", "VC"),
    ("Samoa", "WS"),
    ("San Marino", "SM"),
    ("Sao Tome and Principe", "ST"),
    ("Saudi Arabia", "SA"),
    ("Senegal", "SN"),
    ("Serbia and Montenegro", "CS"),
    ("Seychelles", "SC"),
    ("Sierra Leone", "SL"),
    ("Singapore", "SG"),
    ("Slovakia", "SK"),
    ("Slovenia", "SI"),
    ("Solomon Islands", "SB"),
    ("Somalia", "SO"),
    ("South Africa", "ZA"),
    ("South Georgia and the South Sandwich Islands", "GS"),
    ("Spain", "ES"),
    ("Sri Lanka", "LK"),
    ("Sudan", "SD"),
    ("Suriname", "SR"),
    ("Svalbard and Jan Mayen", "SJ"),
    ("Swaziland", "SZ"),
    ("Sweden", "SE"),
    ("Switzerland", "CH"),
    ("Syrian Arab Republic", "SY"),
    ("Tajikistan", "TJ"),
    ("Tanzania, United Republic of", "TZ"),
    ("Thailand", "TH"),
    ("Timor-Leste", "TL"),
    ("Togo", "TG"),
    ("Tokelau", "TK"),
    ("Tonga", "TO"),
    ("Trinidad and Tobago", "TT"),
    ("Tunisia", "TN"),
    ("Turkey", "TR"),
    ("Turkmenistan", "TM"),
    ("Turks and Caicos Islands", "TC"),
    ("Tuvalu", "TV"),
    ("Uganda", "UG"),
    ("Ukraine", "UA"),
    ("United Arab Emirates", "AE"),
    ("United Kingdom", "GB"),
    ("United States", "US"),
    ("United States Minor Outlying Islands", "UM"),
    ("Uruguay", "UY"),
    ("Uzbekistan", "UZ"),
    ("Vanuatu", "VU"),
    ("Venezuela", "VE"),
    ("Viet Nam", "VN"),
    ("Virgin Islands, British", "VG"),
    ("Virgin Islands, U.S.", "VI"),
    ("Wallis and Futuna", "WF"),
    ("Western Sahara", "EH"),
    ("Yemen", "YE"),
    ("Zambia", "ZM"),
    ("Zimbabwe", "ZW"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct Country {
    name: SharedString,
    code: SharedString,
}

impl Country {
    fn letter_prefix(&self) -> char {
        self.name.chars().next().unwrap_or(' ')
    }
}

impl SelectItem for Country {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn display_title(&self) -> Option<AnyElement> {
        Some(format!("{} ({})", self.name, self.code).into_any_element())
    }

    fn value(&self) -> &SharedString {
        &self.code
    }
}

/// Groups the fixture by initial letter; the list is sorted, so equal
/// prefixes are contiguous and a running group matches itertools chunk_by.
fn country_groups() -> SearchableVec<SelectGroup<Country>> {
    let mut groups: Vec<SelectGroup<Country>> = Vec::new();
    for (name, code) in COUNTRIES {
        let country = Country {
            name: (*name).into(),
            code: (*code).into(),
        };
        let prefix = country.letter_prefix().to_string();
        match groups.last_mut() {
            Some(group) if group.title == prefix => group.items.push(country),
            _ => groups.push(SelectGroup::new(prefix).item(country)),
        }
    }
    SearchableVec::new(groups)
}

pub struct SelectSection {
    disabled: bool,
    size: Size,
    country_select: Entity<SelectState<SearchableVec<SelectGroup<Country>>>>,
    fruit_select: Entity<SelectState<SearchableVec<&'static str>>>,
    simple_select1: Entity<SelectState<Vec<&'static str>>>,
    simple_select2: Entity<SelectState<SearchableVec<&'static str>>>,
    simple_select3: Entity<SelectState<Vec<SharedString>>>,
    menu_max_h_select: Entity<SelectState<Vec<&'static str>>>,
    disabled_select: Entity<SelectState<Vec<SharedString>>>,
    appearance_select: Entity<SelectState<Vec<SharedString>>>,
    input_state: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl SelectSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let country_select = cx.new(|cx| {
            SelectState::new(
                country_groups(),
                Some(IndexPath::default().row(8).section(2)),
                window,
                cx,
            )
            .searchable(true)
        });
        let appearance_select = cx.new(|cx| {
            SelectState::new(
                vec![
                    "CN".into(),
                    "US".into(),
                    "HK".into(),
                    "JP".into(),
                    "KR".into(),
                ],
                Some(IndexPath::default()),
                window,
                cx,
            )
        });
        let input_state = cx.new(|cx| InputState::new(window, cx).placeholder("Your phone number"));

        let fruits = SearchableVec::new(vec![
            "Apple",
            "Orange",
            "Banana",
            "Grape",
            "Pineapple",
            "Watermelon & This is a long long long long long long long long long title",
            "Avocado",
        ]);
        let fruit_select = cx.new(|cx| SelectState::new(fruits, None, window, cx).searchable(true));

        // The Values readout reads this state, so a commit must repaint.
        let _subscriptions =
            vec![cx.subscribe(&country_select, |_, _, _: &SelectEvent<_>, cx| cx.notify())];

        Self {
            disabled: false,
            size: Size::Medium,
            country_select,
            fruit_select,
            simple_select1: cx.new(|cx| {
                SelectState::new(
                    vec![
                        "GPUI", "Iced", "egui", "Makepad", "Slint", "QT", "ImGui", "Cocoa", "WinUI",
                    ],
                    Some(IndexPath::default()),
                    window,
                    cx,
                )
            }),
            simple_select2: cx.new(|cx| {
                let mut select =
                    SelectState::new(SearchableVec::new(vec![]), None, window, cx).searchable(true);

                select.set_items(
                    SearchableVec::new(vec!["Rust", "Go", "C++", "JavaScript"]),
                    window,
                    cx,
                );

                select
            }),
            simple_select3: cx
                .new(|cx| SelectState::new(Vec::<SharedString>::new(), None, window, cx)),
            menu_max_h_select: cx.new(|cx| {
                SelectState::new(
                    vec![
                        "GPUI", "Iced", "egui", "Makepad", "Slint", "QT", "ImGui", "Cocoa", "WinUI",
                    ],
                    Some(IndexPath::default()),
                    window,
                    cx,
                )
            }),
            disabled_select: cx
                .new(|cx| SelectState::new(Vec::<SharedString>::new(), None, window, cx)),
            appearance_select,
            input_state,
            _subscriptions,
        }
    }
}

impl Render for SelectSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = self.size;
        let disabled = self.disabled;

        v_flex()
            .w_full()
            .items_center()
            .gap_4()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                if let Some(size) = action.size() {
                    this.size = size;
                    cx.notify();
                } else if matches!(action, DemoToggle::Disabled) {
                    this.disabled = !this.disabled;
                    cx.notify();
                }
            }))
            .child(demo_toolbar(vec![
                size_dropdown("select-size", size).into_any_element(),
                DropdownButton::new("select-options")
                    .button(Button::new("select-options-trigger").label("Options"))
                    .dropdown_menu(move |menu, _, _| {
                        menu.menu_with_check("Disabled", disabled, Box::new(DemoToggle::Disabled))
                    })
                    .into_any_element(),
            ]))
            .child(
                section("select-search-and-clear", "Search and clear")
                    .description("Search options and clear the value.")
                    .w(rems(17.5))
                    .child(
                        Select::new(&self.country_select)
                            .w(rems(17.5))
                            .with_size(size)
                            .search_placeholder("Search…")
                            .cleanable(true)
                            .disabled(self.disabled),
                    ),
            )
            .child(
                section("select-menu-width", "Menu width")
                    .description("Set trigger and menu widths independently.")
                    .w(rems(17.5))
                    .child(
                        Select::new(&self.fruit_select)
                            .with_size(size)
                            .disabled(self.disabled)
                            .icon(IconName::Search)
                            .w(rems(17.5))
                            .menu_width(rems(25.)),
                    ),
            )
            .child(
                section("select-disabled", "Disabled")
                    .description("Keep the selected value visible.")
                    .w(rems(17.5))
                    .child(
                        Select::new(&self.disabled_select)
                            .w(rems(17.5))
                            .with_size(size)
                            .disabled(true),
                    ),
            )
            .child(
                section("select-title-prefix", "Title prefix")
                    .description("Prefix the selected value.")
                    .w(rems(17.5))
                    .child(
                        Select::new(&self.simple_select1)
                            .w(rems(17.5))
                            .with_size(size)
                            .disabled(self.disabled)
                            .placeholder("UI")
                            .title_prefix("UI: "),
                    ),
            )
            .child(
                section("select-menu-height", "Menu height")
                    .description("Limit the popup height.")
                    .w(rems(17.5))
                    .child(
                        Select::new(&self.menu_max_h_select)
                            .w(rems(17.5))
                            .with_size(size)
                            .disabled(self.disabled)
                            .placeholder("UI")
                            .title_prefix("UI: ")
                            .menu_max_h(rems(6.)),
                    ),
            )
            .child(
                section("select-search", "Search")
                    .description("Filter options from the popup.")
                    .w(rems(17.5))
                    .child(
                        Select::new(&self.simple_select2)
                            .accessibility_label("Programming language")
                            .w(rems(17.5))
                            .with_size(size)
                            .disabled(self.disabled)
                            .placeholder("Language")
                            .title_prefix("Language: "),
                    ),
            )
            .child(
                section("select-empty", "Empty")
                    .description("Render a custom empty state.")
                    .w(rems(17.5))
                    .child(
                        Select::new(&self.simple_select3)
                            .w(rems(17.5))
                            .with_size(size)
                            .disabled(self.disabled)
                            .empty(|_, cx| {
                                h_flex()
                                    .h_24()
                                    .justify_center()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("No Data")
                            }),
                    ),
            )
            .child(
                section("select-custom-appearance", "Custom appearance")
                    .description("Compose an appearance-free select with another control.")
                    .w(rems(17.5))
                    .child(
                        h_flex()
                            .border_1()
                            .border_color(cx.theme().input)
                            .rounded(cx.theme().radius_lg)
                            .text_color(cx.theme().secondary_foreground)
                            .w(rems(17.5))
                            .gap_1()
                            .child(
                                div().w(rems(8.75)).child(
                                    Select::new(&self.appearance_select)
                                        .with_size(size)
                                        .appearance(false)
                                        .py_2()
                                        .pl_3(),
                                ),
                            )
                            .child(Separator::vertical())
                            .child(
                                div().flex_1().child(
                                    Input::new(&self.input_state)
                                        .appearance(false)
                                        .pr_3()
                                        .py_2(),
                                ),
                            )
                            .child(
                                div().p_2().child(
                                    Button::new("select-send")
                                        .with_size(size)
                                        .ghost()
                                        .label("Send"),
                                ),
                            ),
                    ),
            )
            .child(
                section("select-values", "Values")
                    .description("Read selected values from state.")
                    .w_128()
                    .child(
                        v_flex()
                            .gap_3()
                            .child(format!(
                                "Country: {:?}",
                                self.country_select.read(cx).selected_value()
                            ))
                            .child(format!(
                                "fruit: {:?}",
                                self.fruit_select.read(cx).selected_value()
                            ))
                            .child(format!(
                                "UI: {:?}",
                                self.simple_select1.read(cx).selected_value()
                            ))
                            .child(format!(
                                "Language: {:?}",
                                self.simple_select2.read(cx).selected_value()
                            ))
                            .child("This is other text."),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "select",
        "Select",
        "Displays a list of options for the user to pick from, triggered by a button.",
        SelectSection::view(window, cx),
    ));
}
