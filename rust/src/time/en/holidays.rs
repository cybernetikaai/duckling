//! `holidays` rule builders (split from the en Time monolith).

use super::*;

/// Fixed-date / nth-weekday / last-weekday holidays (port of mkRuleHolidays).
/// Computed/lunar holidays (Easter, Chinese NY, …) need precomputed tables and
/// are out of scope here. dow: Mon=1..Sun=7.
pub(super) fn holiday_rules() -> Vec<Rule> {
    vec![
        // Fixed dates, year over year (port of rulePeriodicHolidays monthDay entries).
        holiday_rule("Africa Day", r"africa(n (freedom|liberation))? day", || {
            month_day_td(5, 25)
        }),
        holiday_rule(
            "Africa Industrialization Day",
            r"africa industrialization day",
            || month_day_td(11, 20),
        ),
        holiday_rule("All Saints' Day", r"all saints' day", || {
            month_day_td(11, 1)
        }),
        holiday_rule("All Souls' Day", r"all souls' day", || month_day_td(11, 2)),
        holiday_rule("April Fools", r"(april|all) fool'?s('? day)?", || {
            month_day_td(4, 1)
        }),
        holiday_rule("Arabic Language Day", r"arabic language day", || {
            month_day_td(12, 18)
        }),
        holiday_rule("Assumption of Mary", r"assumption of mary", || {
            month_day_td(8, 15)
        }),
        holiday_rule("Boxing Day", r"boxing day", || month_day_td(12, 26)),
        holiday_rule("Chinese Language Day", r"chinese language day", || {
            month_day_td(4, 20)
        }),
        holiday_rule("Christmas", r"(xmas|christmas)( day)?", || {
            month_day_td(12, 25)
        }),
        holiday_rule("Christmas Eve", r"(xmas|christmas)( day)?('s)? eve", || {
            month_day_td(12, 24)
        }),
        holiday_rule(
            "Day of Remembrance for all Victims of Chemical Warfare",
            r"day of remembrance for all victims of chemical warfare",
            || month_day_td(4, 29),
        ),
        holiday_rule(
            "Day of Remembrance of the Victims of the Rwanda Genocide",
            r"day of remembrance of the victims of the rwanda genocide",
            || month_day_td(4, 7),
        ),
        holiday_rule("Day of the Seafarer", r"day of the seafarer", || {
            month_day_td(6, 25)
        }),
        holiday_rule("Earth Day", r"earth day", || month_day_td(4, 22)),
        holiday_rule("English Language Day", r"english language day", || {
            month_day_td(4, 23)
        }),
        holiday_rule("Epiphany", r"Epiphany", || month_day_td(1, 6)),
        holiday_rule(
            "Feast of St Francis of Assisi",
            r"feast of st\.? francis of assisi",
            || month_day_td(10, 4),
        ),
        holiday_rule(
            "Feast of the Immaculate Conception",
            r"feast of the immaculate conception",
            || month_day_td(12, 8),
        ),
        holiday_rule("Global Day of Parents", r"global day of parents", || {
            month_day_td(6, 1)
        }),
        holiday_rule("Halloween", r"hall?owe?en( day)?", || month_day_td(10, 31)),
        holiday_rule("Human Rights Day", r"human rights? day", || {
            month_day_td(12, 10)
        }),
        holiday_rule(
            "International Albinism Awareness Day",
            r"international albinism awareness day",
            || month_day_td(6, 13),
        ),
        holiday_rule(
            "International Anti-Corruption Day",
            r"international anti(\-|\s)corruption day",
            || month_day_td(12, 9),
        ),
        holiday_rule(
            "International Asteroid Day",
            r"international asteroid day",
            || month_day_td(6, 30),
        ),
        holiday_rule(
            "International Celebrate Bisexuality Day",
            r"international celebrate bisexuality day",
            || month_day_td(9, 23),
        ),
        holiday_rule(
            "International Chernobyl Disaster Remembrance Day",
            r"international chernobyl disaster remembrance day",
            || month_day_td(4, 26),
        ),
        holiday_rule(
            "International Civil Aviation Day",
            r"international civil aviation day",
            || month_day_td(12, 7),
        ),
        holiday_rule(
            "International Customs Day",
            r"international customs day",
            || month_day_td(1, 26),
        ),
        holiday_rule(
            "International Day Against Drug Abuse and Illicit Trafficking",
            r"international day against drug abuse and illicit trafficking",
            || month_day_td(6, 26),
        ),
        holiday_rule(
            "International Day against Nuclear Tests",
            r"international day against nuclear tests",
            || month_day_td(8, 29),
        ),
        holiday_rule(
            "International Day for Biological Diversity",
            r"international day for biological diversity|world biodiversity day",
            || month_day_td(5, 22),
        ),
        holiday_rule(
            "International Day for Monuments and Sites",
            r"international day for monuments and sites",
            || month_day_td(4, 18),
        ),
        holiday_rule(
            "International Day for Preventing the Exploitation of the Environment in War and Armed Conflict",
            r"international day for preventing the exploitation of the environment in war and armed conflict",
            || month_day_td(11, 6),
        ),
        holiday_rule(
            "International Day for South-South Cooperation",
            r"international day for south(\-|\s)south cooperation",
            || month_day_td(9, 12),
        ),
        holiday_rule(
            "International Day for Tolerance",
            r"international day for tolerance",
            || month_day_td(11, 16),
        ),
        holiday_rule(
            "International Day for the Abolition of Slavery",
            r"international day for the abolition of slavery",
            || month_day_td(12, 2),
        ),
        holiday_rule(
            "International Day for the Elimination of Racial Discrimination",
            r"international day for the elimination of racial discrimination",
            || month_day_td(3, 21),
        ),
        holiday_rule(
            "International Day for the Elimination of Sexual Violence in Conflict",
            r"international day for the elimination of sexual violence in conflict",
            || month_day_td(6, 19),
        ),
        holiday_rule(
            "International Day for the Elimination of Violence against Women",
            r"international day for the elimination of violence against women",
            || month_day_td(11, 25),
        ),
        holiday_rule(
            "International Day for the Eradication of Poverty",
            r"international day for the eradication of poverty",
            || month_day_td(10, 17),
        ),
        holiday_rule(
            "International Day for the Preservation of the Ozone Layer",
            r"international day for the preservation of the ozone Layer",
            || month_day_td(9, 16),
        ),
        holiday_rule(
            "International Day for the Remembrance of the Slave Trade and its Abolition",
            r"international day for the remembrance of the slave trade and its abolition",
            || month_day_td(8, 23),
        ),
        holiday_rule(
            "International Day for the Right to the Truth concerning Gross Human Rights Violations and for the Dignity of Victims",
            r"international day for the right to the truth concerning gross human rights violations and for the dignity of victims",
            || month_day_td(3, 24),
        ),
        holiday_rule(
            "International Day for the Total Elimination of Nuclear Weapons",
            r"international day for the total elimination of nuclear weapons",
            || month_day_td(9, 26),
        ),
        holiday_rule(
            "International Day in Support of Victims of Torture",
            r"international day in support of victims of torture",
            || month_day_td(6, 26),
        ),
        holiday_rule(
            "International Day of Charity",
            r"international day of charity",
            || month_day_td(9, 5),
        ),
        holiday_rule(
            "International Day of Commemoration in Memory of the Victims of the Holocaust",
            r"international day of commemoration in memory of the victims of the holocaust",
            || month_day_td(1, 27),
        ),
        holiday_rule(
            "International Day of Democracy",
            r"international day of democracy",
            || month_day_td(9, 15),
        ),
        holiday_rule(
            "International Day of Disabled Persons",
            r"international day of disabled persons",
            || month_day_td(12, 3),
        ),
        holiday_rule(
            "International Day of Families",
            r"international day of families",
            || month_day_td(5, 15),
        ),
        holiday_rule(
            "International Day of Family Remittances",
            r"international day of family remittances",
            || month_day_td(6, 16),
        ),
        holiday_rule(
            "International Day of Forests",
            r"international day of forests",
            || month_day_td(3, 21),
        ),
        holiday_rule(
            "International Day of Friendship",
            r"international day of friendship",
            || month_day_td(7, 30),
        ),
        holiday_rule(
            "International Day of Happiness",
            r"international day of happiness",
            || month_day_td(3, 20),
        ),
        holiday_rule(
            "International Day of Human Space Flight",
            r"international day of human space flight",
            || month_day_td(4, 12),
        ),
        holiday_rule(
            "International Day of Innocent Children Victims of Aggression",
            r"international day of innocent children victims of aggression",
            || month_day_td(6, 4),
        ),
        holiday_rule(
            "International Day of Non-Violence",
            r"international day of non(\-|\s)violence",
            || month_day_td(10, 2),
        ),
        holiday_rule(
            "International Day of Nowruz",
            r"international day of nowruz",
            || month_day_td(3, 21),
        ),
        holiday_rule(
            "International Day of Older Persons",
            r"international day of older persons",
            || month_day_td(10, 1),
        ),
        holiday_rule(
            "International Day of Peace",
            r"international day of peace",
            || month_day_td(9, 21),
        ),
        holiday_rule(
            "International Day of Persons with Disabilities",
            r"international day of persons with disabilities",
            || month_day_td(12, 3),
        ),
        holiday_rule(
            "International Day of Remembrance of Slavery Victims and the Transatlantic Slave Trade",
            r"international day of remembrance of slavery victims and the transatlantic slave trade",
            || month_day_td(3, 25),
        ),
        holiday_rule(
            "International Day of Rural Women",
            r"international day of rural women",
            || month_day_td(10, 15),
        ),
        holiday_rule(
            "International Day of Solidarity with Detained and Missing Staff Members",
            r"international day of solidarity with detained and missing staff members",
            || month_day_td(3, 25),
        ),
        holiday_rule(
            "International Day of Solidarity with the Palestinian People",
            r"international day of solidarity with the palestinian people",
            || month_day_td(11, 29),
        ),
        holiday_rule(
            "International Day of Sport for Development and Peace",
            r"international day of sport for development and peace",
            || month_day_td(4, 6),
        ),
        holiday_rule(
            "International Day of United Nations Peacekeepers",
            r"international day of united nations peacekeepers",
            || month_day_td(5, 29),
        ),
        holiday_rule(
            "International Day of Women and Girls in Science",
            r"international day of women and girls in science",
            || month_day_td(2, 11),
        ),
        holiday_rule(
            "International Day of Yoga",
            r"international day of yoga",
            || month_day_td(6, 21),
        ),
        holiday_rule(
            "International Day of Zero Tolerance for Female Genital Mutilation",
            r"international day of zero tolerance for female genital mutilation",
            || month_day_td(2, 6),
        ),
        holiday_rule(
            "International Day of the Girl Child",
            r"international day of the girl child",
            || month_day_td(10, 11),
        ),
        holiday_rule(
            "International Day of the Victims of Enforced Disappearances",
            r"international day of the victims of enforced disappearances",
            || month_day_td(8, 30),
        ),
        holiday_rule(
            "International Day of the World's Indigenous People",
            r"international day of the world'?s indigenous people",
            || month_day_td(8, 9),
        ),
        holiday_rule(
            "International Day to End Impunity for Crimes against Journalists",
            r"international day to end impunity for crimes against journalists",
            || month_day_td(11, 2),
        ),
        holiday_rule(
            "International Day to End Obstetric Fistula",
            r"international day to end obstetric fistula",
            || month_day_td(5, 23),
        ),
        holiday_rule(
            "International Day for Disaster Reduction",
            r"iddr|international day for (natural )?disaster reduction",
            || month_day_td(10, 13),
        ),
        holiday_rule(
            "International Human Solidarity Day",
            r"international human solidarity day",
            || month_day_td(12, 20),
        ),
        holiday_rule("International Jazz Day", r"international jazz day", || {
            month_day_td(4, 30)
        }),
        holiday_rule(
            "International Literacy Day",
            r"international literacy day",
            || month_day_td(9, 8),
        ),
        holiday_rule(
            "International Men's Day",
            r"international men'?s day",
            || month_day_td(11, 19),
        ),
        holiday_rule(
            "International Migrants Day",
            r"international migrants day",
            || month_day_td(12, 18),
        ),
        holiday_rule(
            "International Mother Language Day",
            r"international mother language day",
            || month_day_td(2, 21),
        ),
        holiday_rule(
            "International Mountain Day",
            r"international mountain day",
            || month_day_td(12, 11),
        ),
        holiday_rule(
            "International Nurses Day",
            r"international nurses day",
            || month_day_td(5, 12),
        ),
        holiday_rule(
            "International Overdose Awareness Day",
            r"international overdose awareness day",
            || month_day_td(8, 31),
        ),
        holiday_rule(
            "International Volunteer Day for Economic and Social Development",
            r"international volunteer day for economic and social development",
            || month_day_td(12, 5),
        ),
        holiday_rule(
            "International Widows' Day",
            r"international widows'? day",
            || month_day_td(6, 23),
        ),
        holiday_rule(
            "International Women's Day",
            r"international women'?s day",
            || month_day_td(3, 8),
        ),
        holiday_rule(
            "International Youth Day",
            r"international youth day",
            || month_day_td(8, 12),
        ),
        holiday_rule("May Day", r"may day", || month_day_td(5, 1)),
        holiday_rule("Nelson Mandela Day", r"nelson mandela day", || {
            month_day_td(7, 18)
        }),
        holiday_rule("New Year's Day", r"new year'?s?( day)?", || {
            month_day_td(1, 1)
        }),
        holiday_rule("New Year's Eve", r"new year'?s? eve", || {
            month_day_td(12, 31)
        }),
        holiday_rule("Orthodox Christmas Day", r"orthodox christmas day", || {
            month_day_td(1, 7)
        }),
        holiday_rule("Orthodox New Year", r"orthodox new year", || {
            month_day_td(1, 14)
        }),
        holiday_rule("Public Service Day", r"public service day", || {
            month_day_td(6, 23)
        }),
        holiday_rule(
            "St. George's Day",
            r"(saint|st\.?) george'?s day|feast of saint george",
            || month_day_td(4, 23),
        ),
        holiday_rule(
            "St Patrick's Day",
            r"(saint|st\.?) (patrick|paddy)'?s day",
            || month_day_td(3, 17),
        ),
        holiday_rule("St. Stephen's Day", r"(saint|st\.?) stephen'?s day", || {
            month_day_td(12, 26)
        }),
        holiday_rule(
            "Time of Remembrance and Reconciliation for Those Who Lost Their Lives during the Second World War",
            r"time of remembrance and reconciliation for those who lost their lives during the second world war",
            || month_day_td(5, 8),
        ),
        holiday_rule("United Nations Day", r"united nations day", || {
            month_day_td(10, 24)
        }),
        holiday_rule(
            "United Nations' Mine Awareness Day",
            r"united nations'? mine awareness day",
            || month_day_td(4, 4),
        ),
        holiday_rule(
            "United Nations' World Health Day",
            r"united nations'? world health day",
            || month_day_td(4, 7),
        ),
        holiday_rule(
            "Universal Children's Day",
            r"universal children'?s day",
            || month_day_td(11, 20),
        ),
        holiday_rule("Valentine's Day", r"valentine'?s?( day)?", || {
            month_day_td(2, 14)
        }),
        holiday_rule("World AIDS Day", r"world aids day", || month_day_td(12, 1)),
        holiday_rule(
            "World Autism Awareness Day",
            r"world autism awareness day",
            || month_day_td(4, 2),
        ),
        holiday_rule(
            "World Autoimmune Arthritis Day",
            r"world autoimmune arthritis day",
            || month_day_td(5, 20),
        ),
        holiday_rule("World Blood Donor Day", r"world blood donor day", || {
            month_day_td(6, 14)
        }),
        holiday_rule(
            "World Book and Copyright Day",
            r"world book and copyright day",
            || month_day_td(4, 23),
        ),
        holiday_rule("World Braille Day", r"world braille day", || {
            month_day_td(1, 4)
        }),
        holiday_rule("World Cancer Day", r"world cancer day", || {
            month_day_td(2, 4)
        }),
        holiday_rule("World Cities Day", r"world cities day", || {
            month_day_td(10, 31)
        }),
        holiday_rule("World CP Day", r"world (cerebral palsy| cp) day", || {
            month_day_td(10, 6)
        }),
        holiday_rule(
            "World Day Against Child Labour",
            r"world day against child labour",
            || month_day_td(6, 12),
        ),
        holiday_rule(
            "World Day against Trafficking in Persons",
            r"world day against trafficking in persons",
            || month_day_td(7, 30),
        ),
        holiday_rule(
            "World Day for Audiovisual Heritage",
            r"world day for audiovisual heritage",
            || month_day_td(10, 27),
        ),
        holiday_rule(
            "World Day for Cultural Diversity for Dialogue and Development",
            r"world day for cultural diversity for dialogue and development",
            || month_day_td(5, 21),
        ),
        holiday_rule(
            "World Day for Safety and Health at Work",
            r"world day for safety and health at work",
            || month_day_td(4, 28),
        ),
        holiday_rule(
            "World Day for the Abolition of Slavery",
            r"world day for the abolition of slavery",
            || month_day_td(12, 2),
        ),
        holiday_rule(
            "World Day of Social Justice",
            r"world day of social justice",
            || month_day_td(2, 20),
        ),
        holiday_rule("World Day of the Sick", r"world day of the sick", || {
            month_day_td(2, 11)
        }),
        holiday_rule(
            "World Day to Combat Desertification and Drought",
            r"world day to combat desertification and drought",
            || month_day_td(6, 17),
        ),
        holiday_rule(
            "World Development Information Day",
            r"world development information day",
            || month_day_td(10, 24),
        ),
        holiday_rule("World Diabetes Day", r"world diabetes day", || {
            month_day_td(11, 14)
        }),
        holiday_rule(
            "World Down Syndrome Day",
            r"world down syndrome day",
            || month_day_td(3, 21),
        ),
        holiday_rule(
            "World Elder Abuse Awareness Day",
            r"world elder abuse awareness day",
            || month_day_td(6, 15),
        ),
        holiday_rule("World Environment Day", r"world environment day", || {
            month_day_td(6, 5)
        }),
        holiday_rule("World Food Day", r"world food day", || month_day_td(10, 16)),
        holiday_rule(
            "World Genocide Commemoration Day",
            r"world genocide commemoration day",
            || month_day_td(12, 9),
        ),
        holiday_rule("World Heart Day", r"world heart day", || {
            month_day_td(9, 29)
        }),
        holiday_rule("World Hepatitis Day", r"world hepatitis day", || {
            month_day_td(7, 28)
        }),
        holiday_rule("World Humanitarian Day", r"world humanitarian day", || {
            month_day_td(8, 19)
        }),
        holiday_rule(
            "World Information Society Day",
            r"world information society day",
            || month_day_td(5, 17),
        ),
        holiday_rule(
            "World Intellectual Property Day",
            r"world intellectual property day",
            || month_day_td(4, 26),
        ),
        holiday_rule("World Malaria Day", r"world malaria day", || {
            month_day_td(4, 25)
        }),
        holiday_rule(
            "World Mental Health Day",
            r"world mental health day",
            || month_day_td(10, 10),
        ),
        holiday_rule(
            "World Meteorological Day",
            r"world meteorological day",
            || month_day_td(3, 23),
        ),
        holiday_rule("World No Tobacco Day", r"world no tobacco day", || {
            month_day_td(5, 31)
        }),
        holiday_rule("World Oceans Day", r"world oceans day", || {
            month_day_td(6, 8)
        }),
        holiday_rule(
            "World Ovarian Cancer Day",
            r"world ovarian cancer day",
            || month_day_td(5, 8),
        ),
        holiday_rule("World Pneumonia Day", r"world pneumonia day", || {
            month_day_td(11, 12)
        }),
        holiday_rule("World Poetry Day", r"world poetry day", || {
            month_day_td(3, 21)
        }),
        holiday_rule("World Population Day", r"world population day", || {
            month_day_td(7, 11)
        }),
        holiday_rule("World Post Day", r"world post day", || month_day_td(10, 9)),
        holiday_rule("World Prematurity Day", r"world prematurity day", || {
            month_day_td(11, 17)
        }),
        holiday_rule(
            "World Press Freedom Day",
            r"world press freedom day",
            || month_day_td(5, 3),
        ),
        holiday_rule("World Rabies Day", r"world rabies day", || {
            month_day_td(9, 28)
        }),
        holiday_rule("World Radio Day", r"world radio day", || {
            month_day_td(2, 13)
        }),
        holiday_rule("World Refugee Day", r"world refugee day", || {
            month_day_td(6, 20)
        }),
        holiday_rule(
            "World Science Day for Peace and Development",
            r"world science day for peace and development",
            || month_day_td(11, 10),
        ),
        holiday_rule(
            "World Sexual Health Day",
            r"world sexual health day",
            || month_day_td(9, 4),
        ),
        holiday_rule("World Soil Day", r"world soil day", || month_day_td(12, 5)),
        holiday_rule("World Stroke Day", r"world stroke day", || {
            month_day_td(10, 29)
        }),
        holiday_rule(
            "World Suicide Prevention Day",
            r"world suicide prevention day",
            || month_day_td(9, 10),
        ),
        holiday_rule("World Teachers' Day", r"world teachers'? day", || {
            month_day_td(10, 5)
        }),
        holiday_rule("World Television Day", r"world television day", || {
            month_day_td(11, 21)
        }),
        holiday_rule("World Toilet Day", r"world toilet day", || {
            month_day_td(11, 19)
        }),
        holiday_rule("World Tourism Day", r"world tourism day", || {
            month_day_td(9, 27)
        }),
        holiday_rule("World Tuberculosis Day", r"world tuberculosis day", || {
            month_day_td(3, 24)
        }),
        holiday_rule("World Tuna Day", r"world tuna day", || month_day_td(5, 2)),
        holiday_rule("World Vegan Day", r"world vegan day", || {
            month_day_td(11, 1)
        }),
        holiday_rule("World Vegetarian Day", r"world vegetarian day", || {
            month_day_td(10, 1)
        }),
        holiday_rule("World Water Day", r"world water day", || {
            month_day_td(3, 22)
        }),
        holiday_rule("World Wetlands Day", r"world wetlands day", || {
            month_day_td(2, 2)
        }),
        holiday_rule("World Wildlife Day", r"world wildlife day", || {
            month_day_td(3, 3)
        }),
        holiday_rule("World Youth Skills Day", r"world youth skills day", || {
            month_day_td(7, 15)
        }),
        holiday_rule(
            "Zero Discrimination Day",
            r"zero discrimination day",
            || month_day_td(3, 1),
        ),
        // Fixed day/week/month, year over year (nthDOWOfMonth / predLastOf).
        holiday_rule("Commonwealth Day", r"commonwealth day", || {
            nth_dow_of_month_td(2, 1, 3)
        }),
        holiday_rule(
            "Day of Remembrance for Road Traffic Victims",
            r"(world )?day of remembrance for road traffic victims",
            || nth_dow_of_month_td(3, 7, 11),
        ),
        holiday_rule(
            "International Day of Cooperatives",
            r"international day of co\-?operatives",
            || nth_dow_of_month_td(1, 6, 7),
        ),
        holiday_rule(
            "Martin Luther King's Day",
            r"(MLK|Martin Luther King('?s)?,?)( Jr\.?| Junior)? day|(civil|idaho human) rights day",
            || nth_dow_of_month_td(3, 1, 1),
        ),
        holiday_rule("World Habitat Day", r"world habitat day", || {
            nth_dow_of_month_td(1, 1, 10)
        }),
        holiday_rule("World Kidney Day", r"world kidney day", || {
            nth_dow_of_month_td(2, 4, 3)
        }),
        holiday_rule("World Leprosy Day", r"world leprosy day", || {
            last_dow_of_month_td(7, 1)
        }),
        holiday_rule("World Maritime Day", r"world maritime day", || {
            last_dow_of_month_td(4, 9)
        }),
        holiday_rule(
            "World Migratory Bird Day",
            r"world migratory bird day",
            || nth_dow_of_month_td(2, 6, 5),
        ),
        holiday_rule("World Philosophy Day", r"world philosophy day", || {
            nth_dow_of_month_td(3, 4, 11)
        }),
        holiday_rule("World Religion Day", r"world religion day", || {
            nth_dow_of_month_td(3, 7, 1)
        }),
        holiday_rule("World Sight Day", r"world sight day", || {
            nth_dow_of_month_td(2, 4, 10)
        }),
        // The day after Thanksgiving (4th Thursday of November + 1 day).
        holiday_rule("Black Friday", r"black frid?day", || {
            cycle_nth_after_td(false, Grain::Day, 1, &nth_dow_of_month_td(4, 4, 11))
        }),
        // Thanksgiving Day is corpus-exercised (EN/US Rules.hs); kept from the
        // prior baseline so its passing cases do not regress.
        holiday_rule("Thanksgiving Day", r"thanks?giving( day)?", || {
            nth_dow_of_month_td(4, 4, 11)
        }),
    ]
}

/// Region-specific holidays (GB/CA/AU/…), built from `fixtures/region_holidays.json`
/// (extracted from Duckling's per-region Rules.hs). US adds none — its holidays are
/// in the base table. Four date kinds are supported: fixed month/day, nth (or last,
/// n=-1) weekday of month, fixed-date interval, and easter-relative offset. The 9
/// "other" entries (Islamic-calendar, nth-relative-to-a-date, conditional) are
/// skipped — documented as a known gap in the progress doc.
pub(super) fn region_holiday_rules(locale: Locale) -> Vec<Rule> {
    let region = match locale {
        Locale::EnUs => "US",
        Locale::EnGb => "GB",
        Locale::EnCa => "CA",
        Locale::EnAu => "AU",
        Locale::EnNz => "NZ",
        Locale::EnIn => "IN",
        Locale::EnIe => "IE",
        Locale::EnZa => "ZA",
        Locale::EnPh => "PH",
        Locale::EnBz => "BZ",
        Locale::EnJm => "JM",
        Locale::EnTt => "TT",
    };
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/region_holidays.json"))
            .expect("region_holidays fixture");
    let Some(arr) = data.get(region).and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let mut rules = Vec::new();
    for h in arr {
        let name = h["name"].as_str().unwrap().to_string();
        let re = h["regex"].as_str().unwrap().to_string();
        let i = |k: &str| h[k].as_i64().unwrap();
        // A constructor for the holiday's TimeData, by date kind. Returns None to
        // skip (unsupported "other" kinds).
        let make: Box<dyn Fn() -> Option<TimeData>> = match h["kind"].as_str().unwrap() {
            "month_day" => {
                let (m, d) = (i("month"), i("day"));
                Box::new(move || Some(month_day_td(m, d)))
            }
            "nth_dow_of_month" => {
                let (n, dow, m) = (i("n"), i("dow"), i("month"));
                Box::new(move || {
                    Some(if n == -1 {
                        last_dow_of_month_td(dow, m)
                    } else {
                        nth_dow_of_month_td(n, dow, m)
                    })
                })
            }
            "interval" => {
                let f = h["from"].as_array().unwrap();
                let t = h["to"].as_array().unwrap();
                let (m1, d1) = (f[0].as_i64().unwrap(), f[1].as_i64().unwrap());
                let (m2, d2) = (t[0].as_i64().unwrap(), t[1].as_i64().unwrap());
                // Closed: the oracle's `to` is the last day's end (e.g. Arbor Week
                // Sep 1–7 → to = Sep 8), i.e. the 2nd endpoint is inclusive.
                Box::new(move || {
                    interval_td(
                        IntervalType::Closed,
                        &month_day_td(m1, d1),
                        &month_day_td(m2, d2),
                    )
                })
            }
            "easter_offset" => {
                let days = i("days");
                Box::new(move || Some(crate::time::computed::easter_shift_td(days)))
            }
            "days_after_nth_dow" => {
                // N days from an nth-weekday-of-month anchor: Election Day = 1 day
                // after 1st Mon Nov; Cyber Monday = 4 days after Thanksgiving.
                let (days, n, dow, m) = (i("days"), i("n"), i("dow"), i("month"));
                Box::new(move || {
                    Some(cycle_nth_after_td(
                        false,
                        Grain::Day,
                        days,
                        &nth_dow_of_month_td(n, dow, m),
                    ))
                })
            }
            "nth_dow_rel_date" => {
                let (n, dow, m, d) = (i("n"), i("dow"), i("month"), i("day"));
                Box::new(move || Some(nth_dow_rel_date_td(n, dow, m, d)))
            }
            _ => continue, // remaining "other": intervals / calendar-computed / etc.
        };
        rules.push(Rule {
            name: format!("holiday[{region}]: {name}"),
            pattern: vec![PatternItem::Regex(compile(&re))],
            prod: Box::new(move |_| make().map(|td| Token::Time(mk_holiday(&name, td)))),
        });
    }
    rules
}

/// Holidays absent from Duckling's frozen (~2020-03) holiday data — whether
/// introduced/renamed after that freeze (Juneteenth National Independence Day,
/// King's Birthday) or simply never included in Duckling's tables (Australia's
/// Queen's/King's Birthday). These deliberately DIVERGE from the Duckling
/// oracle, which returns nothing for them — they are verified against official
/// government dates rather than the oracle, and kept separate from the faithful
/// `region_holiday_rules` port above. Region-scoped, since each is a national
/// holiday of one country. Same calendar math as Duckling's own rules, so they
/// resolve like any other recurring holiday (the parser gives the date for the
/// reference year regardless of when the holiday was established).
pub(super) fn modern_holiday_rules(locale: Locale) -> Vec<Rule> {
    match locale {
        Locale::EnUs => vec![
            // Juneteenth National Independence Day — U.S. federal holiday since
            // June 2021. Duckling already has the June 19 date under the plain
            // "juneteenth" name; this adds the formal federal name (same date).
            holiday_rule(
                "Juneteenth National Independence Day",
                r"juneteenth national independence day",
                || month_day_td(6, 19),
            ),
            // Indigenous Peoples' Day — 2nd Monday of October, same as the
            // fixture's "indigenous people's day" entry, but matching the
            // plural/no-apostrophe spellings that single regex missed.
            holiday_rule(
                "Indigenous Peoples' Day",
                r"indigenous peoples?'? day",
                || nth_dow_of_month_td(2, 1, 10),
            ),
        ],
        Locale::EnCa => vec![
            // National Day for Truth and Reconciliation — Canadian federal
            // statutory holiday since 2021, September 30. Orange Shirt Day is
            // observed on the same date.
            holiday_rule(
                "National Day for Truth and Reconciliation",
                r"(national day for )?truth and reconciliation( day)?|orange shirt day",
                || month_day_td(9, 30),
            ),
            // Emancipation Day — federally designated in Canada since 2021,
            // August 1 (marks the 1834 abolition of slavery across the British
            // Empire). Note: the U.S. "Emancipation Day" is a different date.
            holiday_rule("Emancipation Day", r"emancipation day", || {
                month_day_td(8, 1)
            }),
            // National Indigenous Peoples Day — observed in Canada on June 21.
            holiday_rule(
                "National Indigenous Peoples Day",
                r"national indigenous peoples? day",
                || month_day_td(6, 21),
            ),
        ],
        Locale::EnAu => vec![
            // Queen's Birthday / King's Birthday — a major Australian public
            // holiday Duckling's AU rules never included. Renamed "King's
            // Birthday" nationally after the 2022 accession of Charles III;
            // both names resolve to the same date. Observed on the 2nd Monday
            // of June by most states and territories (NSW, VIC, SA, TAS, NT,
            // ACT) — the majority convention, matching how Duckling picks a
            // single date for AU holidays that vary by state (e.g. Labour Day).
            // QLD (1st Mon Oct) and WA (governor-set) differ and are not
            // represented, same limitation as the faithful port.
            holiday_rule(
                "Queen's Birthday",
                r"(the )?(queen|king|monarch|sovereign)'?s'? (official )?birthday",
                || nth_dow_of_month_td(2, 1, 6),
            ),
        ],
        Locale::EnNz => vec![
            // Matariki (Māori New Year) — NZ public holiday since 2022, on a
            // legislated Friday each year (no weekday rule). Table 2022–2052.
            holiday_rule("Matariki", r"matariki", || {
                crate::time::computed::computed_holiday_td(crate::time::computed::MATARIKI)
            }),
            // King's Birthday — NZ observes the sovereign's birthday on the 1st
            // Monday of June (distinct from AU's 2nd Monday). Duckling has NZ
            // "Queen's Birthday"; this adds the post-2022 King's Birthday rename
            // (same date). Regex excludes "queen" to avoid duplicating the
            // faithful NZ Queen's Birthday rule.
            holiday_rule(
                "King's Birthday",
                r"(the )?(king|monarch|sovereign)'?s'? (official )?birthday",
                || nth_dow_of_month_td(1, 1, 6),
            ),
        ],
        Locale::EnIe => vec![
            // St Brigid's Day / Lá Fhéile Bríde — IE public holiday since 2023:
            // first Monday of February, except 1 Feb when it is a Friday. Exact
            // legislated table (see ST_BRIGIDS_DAY) rather than a weekday rule.
            holiday_rule(
                "St Brigid's Day",
                r"(st\.?|saint) brigid'?s day|l[áa] fh[ée]ile br[íi]de",
                || {
                    crate::time::computed::computed_holiday_td(
                        crate::time::computed::ST_BRIGIDS_DAY,
                    )
                },
            ),
        ],
        _ => Vec::new(),
    }
}
