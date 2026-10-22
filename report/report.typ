#import "template.typ": *
#show: report.with(title: "Звіт з виконання лабораторної роботи №3", author: "Ісаченко Н. С", group: "ФІ-52мн")

Гітхаб репозиторій з проектом: https://github.com/misach2135/distributed_systems_labs/tree/task3/microservices-with-hazelcast

У цій лабораторній роботі `counter-service` було інтегровано з бд PostgreSQL, `logger-service` був переписаний на Python, через потребу інтеграції з Hazelcast Distributed Map.

Запущені сервіси у docker-compose:

#figure(
  image("assets/image-1.png"),
  caption: [Запущені контейнери: facade, counter, три екземпляри logger, кластер Hazelcast з трьох нод, Management Center і PostgreSQL],
)

POST та GET в `facade-service`:

#figure(
  image("assets/image-6.png"),
  caption: [10 транзакцій через `POST /transaction` і читання через `GET /user/user1`: баланс 55 (1 + 2 + … + 10) і всі 10 транзакцій],
)

Логи кожного інстанса `logger-service`:

#figure(
  image("assets/image-5.png"),
  caption: [Логи `logger-1`: отримав транзакції з сумами 3, 5 і 10],
)

#figure(
  image("assets/image-4.png"),
  caption: [Логи `logger-2`: отримав транзакції з сумами 4 і 8],
)

#figure(
  image("assets/image-3.png"),
  caption: [Логи `logger-3`: отримав транзакції з сумами 1, 2, 6, 7 і 9],
)

#figure(
  image("assets/image-7.png"),
  caption: [Запуск циклу, який кожні 2 с надсилає транзакцію `{demo, +1}`; усі запити повертають 200],
)

#figure(
  image("assets/image-8.png"),
  caption: [Після зупинки `logger-1` транзакції `demo` продовжують зберігати `logger-2` і `logger-3`; останні рядки логів `logger-1` — його завершення],
)

#figure(
  image("assets/image-9.png"),
  caption: [Транзакція, збережена `logger-2` після зупинки `logger-1`, у Distributed Map `transactions` (Management Center)],
)

#figure(
  image("assets/image-10.png"),
  caption: [Ще одна транзакція `demo`, збережена після зупинки `logger-1`, у Distributed Map `transactions`],
)

Навантажувальний тест `tests/load_test.py`: 10 клієнтів по 10 000 POST-запитів на +1. Два сценарії: кожен клієнт поповнює свій рахунок (10 accounts) або всі один спільний (1 account).

Порівняно з першою лабою стало повільніше, в основному через `logging-service`: тепер він на Python і пише в Hazelcast з бекапами на інші ноди. `counter-service` теж трохи повільніший, бо пише в PostgreSQL на диск.

#figure(
  table(
  columns: (auto, auto, auto, auto),
  align: left + horizon,
  table.header(
    [*Test scenarios*], [*Task 1 (in-mem)*], [*Task 3 (DB)*], [*Task 5 (final)*]
  ),
  [*10 accounts*],
  [Total time: *36.327 s*],
  [Total time: *82.530 s*],
  [Total time:],

  [],
  [logging-service\ contribution: *224643.4 ms*],
  [logging-service\ contribution: *723977.8 ms*],
  [logging-service\ contribution:],

  [],
  [counter-service\ contribution: *37546.5 ms*],
  [counter-service\ contribution: *94378.3 ms*],
  [counter-service\ contribution:],
  [*1 account*],
  [Total time: *47.917 s*],
  [Total time: *87.422 s*],
  [Total time:],

  [],
  [logging-service\ contribution: *148583.7 ms*],
  [logging-service\ contribution: *761603.3 ms*],
  [logging-service\ contribution:],

  [],
  [counter-service\ contribution: *72849.0 ms*],
  [counter-service\ contribution: *119044.1 ms*],
  [counter-service\ contribution: ]
), caption: [Таблиця порівняння результатів]
) <results_table>
