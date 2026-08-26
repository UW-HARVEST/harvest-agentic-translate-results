/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 * 
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 * 
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */
// inventory.c
#include "inventory.h"

// Define the container implementations
DEFINE_ARRAY(int)
DEFINE_ARRAY(double)
DEFINE_ARRAY(item_t)
DEFINE_ARRAY(order_t)

DEFINE_LIST(int)
DEFINE_LIST(double)
DEFINE_LIST(item_t)
DEFINE_LIST(order_t)

void print_item(item_t item) {
    printf("  [%d] %s\n", item.id, item.name);
    printf("      Category: %s\n", item.category);
    printf("      Price: $%.2f\n", item.price);
    printf("      Quantity: %d\n", item.quantity);
}

void print_order(order_t order) {
    printf("  Order - Customer ID: %d, Name: %s\n", 
           order.customer_id, order.customer_name);
    printf("          Total: $%.2f\n", order.total_amount);
}

item_t create_item(int id, const char *name, const char *category, 
                   double price, int quantity) {
    item_t item;
    item.id = id;
    strncpy(item.name, name, MAX_NAME_LENGTH - 1);
    item.name[MAX_NAME_LENGTH - 1] = '\0';
    strncpy(item.category, category, MAX_CATEGORY_LENGTH - 1);
    item.category[MAX_CATEGORY_LENGTH - 1] = '\0';
    item.price = price;
    item.quantity = quantity;
    return item;
}

order_t create_order(int customer_id, const char *customer_name, 
                     double total_amount) {
    order_t order;
    order.customer_id = customer_id;
    strncpy(order.customer_name, customer_name, MAX_NAME_LENGTH - 1);
    order.customer_name[MAX_NAME_LENGTH - 1] = '\0';
    order.total_amount = total_amount;
    return order;
}

void calculate_inventory_stats(array_item_t_t *items) {
    if (!items || items->size == 0) {
        printf("No items in inventory\n");
        return;
    }
    
    printf("\n=== Inventory Statistics (Array) ===\n");
    
    double total_value = 0.0;
    int total_items = 0;
    double max_price = 0.0;
    double min_price = items->data[0].price;
    
    // Use ARRAY_FOREACH macro
    item_t item;
    ARRAY_FOREACH(item_t, item, items) {
        total_value += item.price * item.quantity;
        total_items += item.quantity;
        if (item.price > max_price) max_price = item.price;
        if (item.price < min_price) min_price = item.price;
    }
    
    printf("Total unique items: %zu\n", items->size);
    printf("Total item count: %d\n", total_items);
    printf("Total inventory value: $%.2f\n", total_value);
    printf("Average item price: $%.2f\n", total_value / total_items);
    printf("Most expensive item: $%.2f\n", max_price);
    printf("Least expensive item: $%.2f\n", min_price);
}

void calculate_order_stats(list_order_t_t *orders) {
    if (!orders || orders->size == 0) {
        printf("No orders to analyze\n");
        return;
    }
    
    printf("\n=== Order Statistics (List) ===\n");
    
    double total_revenue = 0.0;
    double max_order = 0.0;
    double min_order = -1.0;
    
    // Use LIST_FOREACH macro
    order_t order;
    LIST_FOREACH(order_t, order, orders) {
        total_revenue += order.total_amount;
        if (order.total_amount > max_order) {
            max_order = order.total_amount;
        }
        if (min_order < 0 || order.total_amount < min_order) {
            min_order = order.total_amount;
        }
    }
    
    printf("Total orders: %zu\n", orders->size);
    printf("Total revenue: $%.2f\n", total_revenue);
    printf("Average order value: $%.2f\n", total_revenue / orders->size);
    printf("Largest order: $%.2f\n", max_order);
    printf("Smallest order: $%.2f\n", min_order);
}

void find_items_by_category(array_item_t_t *items, const char *category) {
    if (!items || !category) return;
    
    printf("\n=== Items in category '%s' ===\n", category);
    
    int found = 0;
    item_t item;
    
    // Use ARRAY_FOREACH to iterate
    ARRAY_FOREACH(item_t, item, items) {
        if (strcmp(item.category, category) == 0) {
            print_item(item);
            found++;
        }
    }
    
    if (found == 0) {
        printf("No items found in this category\n");
    } else {
        printf("Found %d items\n", found);
    }
}

void find_expensive_items(list_item_t_t *items, double min_price) {
    if (!items) return;
    
    printf("\n=== Items priced above $%.2f ===\n", min_price);
    
    int found = 0;
    item_t item;
    
    // Use LIST_FOREACH to iterate
    LIST_FOREACH(item_t, item, items) {
        if (item.price >= min_price) {
            print_item(item);
            found++;
        }
    }
    
    if (found == 0) {
        printf("No items found above this price\n");
    } else {
        printf("Found %d items\n", found);
    }
}
