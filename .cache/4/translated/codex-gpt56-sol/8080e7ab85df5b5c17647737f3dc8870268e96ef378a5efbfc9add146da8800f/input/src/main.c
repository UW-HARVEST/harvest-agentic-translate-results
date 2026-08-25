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
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stddef.h>
#include "inventory.h"

void print_menu(void) {
    printf("\n");
    printf("========================================\n");
    printf("  GENERIC FOR_EACH MACRO DEMO\n");
    printf("========================================\n");
    printf("1. Demo: Integer Containers\n");
    printf("2. Demo: Double Containers\n");
    printf("3. Demo: Inventory Array\n");
    printf("4. Demo: Order List\n");
    printf("5. Demo: Mixed Operations\n");
    printf("6. Run All Demos\n");
    printf("7. Exit\n");
    printf("========================================\n");
    printf("Choice: ");
}

void demo_integer_containers(void) {
    printf("\n");
    printf("========================================\n");
    printf("  DEMO 1: Integer Containers\n");
    printf("========================================\n");
    
    // Create integer array
    array_int_t *int_array = array_int_create(10);
    printf("\n--- Integer Array ---\n");
    printf("Adding integers: 10, 20, 30, 40, 50\n");
    array_int_push(int_array, 10);
    array_int_push(int_array, 20);
    array_int_push(int_array, 30);
    array_int_push(int_array, 40);
    array_int_push(int_array, 50);
    
    printf("Array contents: ");
    int val;
    ARRAY_FOREACH(int, val, int_array) {
        printf("%d ", val);
    }
    printf("\n");
    
    // Calculate sum using ARRAY_FOREACH
    int sum = 0;
    ARRAY_FOREACH(int, val, int_array) {
        sum += val;
    }
    printf("Sum: %d\n", sum);
    printf("Average: %.2f\n", (double)sum / int_array->size);
    
    // Create integer list
    list_int_t *int_list = list_int_create();
    printf("\n--- Integer List ---\n");
    printf("Adding integers: 100, 200, 300, 400, 500\n");
    list_int_append(int_list, 100);
    list_int_append(int_list, 200);
    list_int_append(int_list, 300);
    list_int_append(int_list, 400);
    list_int_append(int_list, 500);
    
    printf("List contents: ");
    LIST_FOREACH(int, val, int_list) {
        printf("%d ", val);
    }
    printf("\n");
    
    // Calculate product using LIST_FOREACH
    long long product = 1;
    LIST_FOREACH(int, val, int_list) {
        product *= val;
    }
    printf("Product: %lld\n", product);
    
    // Cleanup
    array_int_destroy(int_array);
    list_int_destroy(int_list);
}

void demo_double_containers(void) {
    printf("\n");
    printf("========================================\n");
    printf("  DEMO 2: Double Containers\n");
    printf("========================================\n");
    
    // Create double array
    array_double_t *double_array = array_double_create(5);
    printf("\n--- Double Array (Temperatures in Celsius) ---\n");
    
    double temps[] = {23.5, 25.0, 22.8, 26.3, 24.1, 21.9, 27.5};
    int num_temps = sizeof(temps) / sizeof(temps[0]);
    
    printf("Adding temperatures: ");
    for (int i = 0; i < num_temps; i++) {
        array_double_push(double_array, temps[i]);
        printf("%.1f ", temps[i]);
    }
    printf("\n");
    
    // Find min, max, average using ARRAY_FOREACH
    double min_temp = temps[0];
    double max_temp = temps[0];
    double sum_temp = 0.0;
    double temp;
    
    ARRAY_FOREACH(double, temp, double_array) {
        if (temp < min_temp) min_temp = temp;
        if (temp > max_temp) max_temp = temp;
        sum_temp += temp;
    }
    
    printf("Minimum: %.1f°C\n", min_temp);
    printf("Maximum: %.1f°C\n", max_temp);
    printf("Average: %.1f°C\n", sum_temp / double_array->size);
    
    // Create double list
    list_double_t *price_list = list_double_create();
    printf("\n--- Double List (Product Prices) ---\n");
    
    double prices[] = {9.99, 14.50, 7.25, 22.00, 5.99, 18.75};
    int num_prices = sizeof(prices) / sizeof(prices[0]);
    
    printf("Adding prices: ");
    for (int i = 0; i < num_prices; i++) {
        list_double_append(price_list, prices[i]);
        printf("$%.2f ", prices[i]);
    }
    printf("\n");
    
    // Calculate total and find items under $10 using LIST_FOREACH
    double total = 0.0;
    int count_under_10 = 0;
    
    LIST_FOREACH(double, temp, price_list) {
        total += temp;
        if (temp < 10.0) count_under_10++;
    }
    
    printf("Total cost: $%.2f\n", total);
    printf("Items under $10: %d\n", count_under_10);
    
    // Cleanup
    array_double_destroy(double_array);
    list_double_destroy(price_list);
}

void demo_inventory_array(void) {
    printf("\n");
    printf("========================================\n");
    printf("  DEMO 3: Inventory Array (Items)\n");
    printf("========================================\n");
    
    // Create inventory array
    array_item_t_t *inventory = array_item_t_create(20);
    
    // Add items
    printf("\n--- Adding Items to Inventory ---\n");
    array_item_t_push(inventory, create_item(1, "Laptop", "Electronics", 899.99, 15));
    array_item_t_push(inventory, create_item(2, "Mouse", "Electronics", 24.99, 50));
    array_item_t_push(inventory, create_item(3, "Keyboard", "Electronics", 79.99, 30));
    array_item_t_push(inventory, create_item(4, "Monitor", "Electronics", 299.99, 20));
    array_item_t_push(inventory, create_item(5, "Desk Chair", "Furniture", 199.99, 10));
    array_item_t_push(inventory, create_item(6, "Desk", "Furniture", 349.99, 8));
    array_item_t_push(inventory, create_item(7, "Notebook", "Office", 4.99, 100));
    array_item_t_push(inventory, create_item(8, "Pen Set", "Office", 12.99, 75));
    array_item_t_push(inventory, create_item(9, "USB Cable", "Electronics", 9.99, 60));
    array_item_t_push(inventory, create_item(10, "Bookshelf", "Furniture", 149.99, 12));
    
    printf("Added %zu items to inventory\n", inventory->size);
    
    // Display all items using ARRAY_FOREACH
    printf("\n--- All Inventory Items ---\n");
    item_t item;
    ARRAY_FOREACH(item_t, item, inventory) {
        print_item(item);
        printf("\n");
    }
    
    // Calculate statistics
    calculate_inventory_stats(inventory);
    
    // Find items by category
    find_items_by_category(inventory, "Electronics");
    find_items_by_category(inventory, "Furniture");
    
    // Find low stock items using ARRAY_FOREACH
    printf("\n--- Low Stock Items (< 20) ---\n");
    int low_stock_count = 0;
    ARRAY_FOREACH(item_t, item, inventory) {
        if (item.quantity < 20) {
            print_item(item);
            low_stock_count++;
        }
    }
    printf("Total low stock items: %d\n", low_stock_count);
    
    // Cleanup
    array_item_t_destroy(inventory);
}

void demo_order_list(void) {
    printf("\n");
    printf("========================================\n");
    printf("  DEMO 4: Order List (Orders)\n");
    printf("========================================\n");
    
    // Create order list
    list_order_t_t *orders = list_order_t_create();
    
    // Add orders
    printf("\n--- Adding Orders ---\n");
    list_order_t_append(orders, create_order(1001, "Alice Johnson", 1249.95));
    list_order_t_append(orders, create_order(1002, "Bob Smith", 89.99));
    list_order_t_append(orders, create_order(1003, "Carol White", 549.98));
    list_order_t_append(orders, create_order(1004, "David Brown", 24.99));
    list_order_t_append(orders, create_order(1005, "Eve Davis", 899.99));
    list_order_t_append(orders, create_order(1006, "Frank Miller", 374.97));
    list_order_t_append(orders, create_order(1007, "Grace Lee", 159.98));
    list_order_t_append(orders, create_order(1008, "Henry Wilson", 1099.99));
    
    printf("Added %zu orders\n", orders->size);
    
    // Display all orders using LIST_FOREACH
    printf("\n--- All Orders ---\n");
    order_t order;
    LIST_FOREACH(order_t, order, orders) {
        print_order(order);
    }
    
    // Calculate statistics
    calculate_order_stats(orders);
    
    // Find large orders using LIST_FOREACH
    printf("\n--- Large Orders (> $500) ---\n");
    int large_order_count = 0;
    double large_order_total = 0.0;
    
    LIST_FOREACH(order_t, order, orders) {
        if (order.total_amount > 500.0) {
            print_order(order);
            large_order_count++;
            large_order_total += order.total_amount;
        }
    }
    
    printf("Total large orders: %d\n", large_order_count);
    printf("Revenue from large orders: $%.2f\n", large_order_total);
    
    // Cleanup
    list_order_t_destroy(orders);
}

void demo_mixed_operations(void) {
    printf("\n");
    printf("========================================\n");
    printf("  DEMO 5: Mixed Operations\n");
    printf("========================================\n");
    
    // Create both array and list with items
    array_item_t_t *array_inventory = array_item_t_create(10);
    list_item_t_t *list_inventory = list_item_t_create();
    
    printf("\n--- Populating both Array and List ---\n");
    
    // Add same items to both
    item_t items[] = {
        create_item(1, "Smartphone", "Electronics", 699.99, 25),
        create_item(2, "Tablet", "Electronics", 449.99, 18),
        create_item(3, "Headphones", "Electronics", 149.99, 40),
        create_item(4, "Smart Watch", "Electronics", 299.99, 22),
        create_item(5, "Power Bank", "Electronics", 39.99, 55)
    };
    
    int num_items = sizeof(items) / sizeof(items[0]);
    
    for (int i = 0; i < num_items; i++) {
        array_item_t_push(array_inventory, items[i]);
        list_item_t_append(list_inventory, items[i]);
    }
    
    printf("Added %d items to both containers\n", num_items);
    
    // Compare iteration performance (conceptually)
    printf("\n--- Iterating through Array ---\n");
    int array_count = 0;
    item_t item;
    ARRAY_FOREACH(item_t, item, array_inventory) {
        array_count++;
    }
    printf("Array iteration count: %d\n", array_count);
    
    printf("\n--- Iterating through List ---\n");
    int list_count = 0;
    LIST_FOREACH(item_t, item, list_inventory) {
        list_count++;
    }
    printf("List iteration count: %d\n", list_count);
    
    // Find items above certain price in both
    double price_threshold = 200.0;
    
    printf("\n--- Items above $%.2f (Array) ---\n", price_threshold);
    ARRAY_FOREACH(item_t, item, array_inventory) {
        if (item.price >= price_threshold) {
            printf("  %s: $%.2f\n", item.name, item.price);
        }
    }
    
    printf("\n--- Items above $%.2f (List) ---\n", price_threshold);
    LIST_FOREACH(item_t, item, list_inventory) {
        if (item.price >= price_threshold) {
            printf("  %s: $%.2f\n", item.name, item.price);
        }
    }
    
    // Cleanup
    array_item_t_destroy(array_inventory);
    list_item_t_destroy(list_inventory);
}

int main(void) {
    printf("╔════════════════════════════════════════╗\n");
    printf("║   GENERIC FOR_EACH MACRO DEMO         ║\n");
    printf("║   Demonstrating Generic Containers    ║\n");
    printf("╚════════════════════════════════════════╝\n");
    
    char input[256];
    int choice;
    
    while (1) {
        print_menu();
        
        if (!fgets(input, sizeof(input), stdin)) {
            break;
        }
        
        if (sscanf(input, "%d", &choice) != 1) {
            printf("Invalid input\n");
            continue;
        }
        
        switch (choice) {
            case 1:
                demo_integer_containers();
                break;
                
            case 2:
                demo_double_containers();
                break;
                
            case 3:
                demo_inventory_array();
                break;
                
            case 4:
                demo_order_list();
                break;
                
            case 5:
                demo_mixed_operations();
                break;
                
            case 6:
                printf("\n=== Running All Demos ===\n");
                demo_integer_containers();
                demo_double_containers();
                demo_inventory_array();
                demo_order_list();
                demo_mixed_operations();
                printf("\n========================================\n");
                printf("  All demos completed successfully!\n");
                printf("========================================\n");
                break;
                
            case 7:
                printf("\nGoodbye!\n");
                return 0;
                
            default:
                printf("Invalid choice\n");
                break;
        }
    }
    
    return 0;
}
