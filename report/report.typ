#import "template.typ": *
#show: report.with(title: "Звіт з виконання лабораторної роботи №3", author: "Ісаченко Н. С", group: "ФІ-52мн")

Гітхаб репозиторій з проектом: https://github.com/misach2135/distributed_systems_labs/tree/task3/microservices-with-hazelcast

У цій лабораторній роботі `counter-service` було інтегровано з бд PostgreSQL, `logger-service` був переписаний на Python, через потребу інтеграції з Hazelcast Distributed Map.

#image("assets/image-1.png")

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
  [Total time: *111.986 s*],

  [],
  [logging-service\ contribution: *224643.4 ms*],
  [logging-service\ contribution: *723977.8 ms*],
  [logging-service\ contribution: *755938.6 ms*],

  [],
  [counter-service\ contribution: *37546.5 ms*],
  [counter-service\ contribution: *94378.3 ms*],
  [counter-service\ contribution: *488016.4 ms*],
  [*1 account*],
  [Total time: *47.917 s*],
  [Total time: *87.422 s*],
  [Total time: *109.316 s*],

  [],
  [logging-service\ contribution: *148583.7 ms*],
  [logging-service\ contribution: *761603.3 ms*],
  [logging-service\ contribution: *770365.7 ms*],

  [],
  [counter-service\ contribution: *72849.0 ms*],
  [counter-service\ contribution: *119044.1 ms*],
  [counter-service\ contribution: *423765.3 ms*]
), caption: [Таблиця порівняння результатів]
) <results_table>
