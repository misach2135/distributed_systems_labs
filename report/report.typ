#import "template.typ": *
#show: report.with(title: "Звіт з виконання лабораторної роботи №5", author: "Ісаченко Н. С", group: "ФІ-52мн")

Гітхаб репозиторій з проектом: https://github.com/misach2135/distributed_systems_labs/tree/micro_consul

Завданням лабораторної роботи було використати Consul або Kubernetes в якості
Config Server, Service Registry та Service Discovery.

У цій лабораторній роботі для розгортання мікросервісів використовувався Kubernetes.
Ще з попередніх лабораторних робіт, кожен сервіс запускався у своєму контейнері. Тепер
контейнери запускаються у Pod-ах, а замість реалізованого у 4 лабораторній роботі
config-server-а(який насправді виконував роль service register) використовується EndpointSlices API з Kubernetes.

Усі конфігураційні файли обʼєктів Kubernetes лежать у репозиторії, у гілці `micro_consul`, у папці `infra/k8s`.

#image("assets/image.png")

#image("assets/image_1.png")

#image("assets/image_2.png")

#pagebreak()

POST та GET запити:

#image("assets/image-2.png")

Kafka та Hazelcast конфігуруються через ConfigMap:

```yml
apiVersion: v1
kind: ConfigMap
metadata:
  name: hazelcast-config
  namespace: ds
data:
  HZ_CLUSTER_NAME: lab
  HZ_MEMBERS: hazelcast:5701
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: kafka-config
  namespace: ds
data:
  KAFKA_BOOTSTRAP: kafka:9092
  KAFKA_TOPIC: transactions
```

Тести запускались лише з одни сервісом facade. 

#figure(
  table(
  columns: (auto, auto, auto, auto),
  align: left + horizon,
  table.header(
    [*Test scenarios*], [*Task 1 (in-mem)*], [*Task 3 (DB)*], [*Task 5 (final)*]
  ),
  [*10 accounts*],
  [Total time: *36.327 s*],
  [Total time:],
  [Total time: *111.986 s*],

  [],
  [logging-service\ contribution: *224643.4 ms*],
  [logging-service\ contribution:],
  [logging-service\ contribution: *755938.6 ms*],

  [],
  [counter-service\ contribution: *37546.5 ms*],
  [counter-service\ contribution:],
  [counter-service\ contribution: *488016.4 ms*],
  [*1 account*],
  [Total time: *47.917 s*],
  [Total time:],
  [Total time: *109.316 s*],

  [],
  [logging-service\ contribution: *148583.7 ms*],
  [logging-service\ contribution:],
  [logging-service\ contribution: *770365.7 ms*],

  [],
  [counter-service\ contribution: *72849.0 ms*],
  [counter-service\ contribution:],
  [counter-service\ contribution: *423765.3 ms*]
), caption: [Таблиця порівняння результатів]
) <results_table>
