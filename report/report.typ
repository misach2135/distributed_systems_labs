#import "template.typ": *
#show: report.with(title: "Звіт з виконання лабораторної роботи №4", author: "Ісаченко Н. С", group: "ФІ-52мн")

Гітхаб репозиторій з проектом: https://github.com/misach2135/distributed_systems_labs/tree/micro_mq

В якості messaging queue було обрано Apache Kafka. Як і інші сервіси, Kafka піднімається через docker-compose.

Реалізований сервіс `config-server`, який зберігає у внутрішній памʼяті назви сервісів та їх адреси. Facade, Logger та Counter реєструються у `config-server`, використовуючи `/register` ендпоїнт, і через `/services/{service}` отримуються адреси інстансів сервісів. Перед кожним викликом сервіса, facade бере список адрес через `/services/{service}`, перемішує їх, та намагається викликати, якщо не виходить бере наступну адресу.

Логи `config-server`-а:

#image("assets/image-4.png")

Отримання адрес сервісів через `GET /services{service}`.

#image("assets/image-3.png")

Повідомлення з кафки, які приходять після поповнення акаунту користувача:

#image("assets/image-5.png")

Причому `facade` тепер не повертає баланс одразу, він повертає `txid`. 

Якщо відключити `counter`, то транзакції не загубляться, а будуть висіти в черзі, коли `counter` включиться, то транзакції обробляться. В `counter` за це відповідає таска `consumer::run()`, яка читає записи з `Kafka`, та додає їх у таблицю.

#image("assets/image-6.png")

Логи `counter-service`:

#image("assets/image-7.png")

Логи `facade`:

#image("assets/image-9.png")

Логи з logger через hazelcast:

#image("assets/image-8.png")